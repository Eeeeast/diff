use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use diff_match_patch_rs::{Compat, DiffMatchPatch, Ops, dmp};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Get the diff between two inputs
    Diff {
        /// Left input (app or file or string)
        left: String,
        /// Right input (file or string)
        right: String,
        /// Compare mode
        #[clap(short, long, value_enum, default_value_t = Mode::Interactive)]
        mode: Mode,
    },
    /// Generate example test cases
    Example {
        /// Number of test cases to generate
        #[clap(short, long, default_value_t = 3)]
        #[arg(value_parser = clap::value_parser!(u8).range(1..))]
        count: u8,
        /// Output file path (prints to stdout if not provided)
        path: Option<std::path::PathBuf>,
    },
}

#[derive(ValueEnum, Clone)]
enum Mode {
    /// Run the program with test cases from a YAML file
    Program,
    /// Compare directly entered strings
    Interactive,
    /// Compare contents of specified files
    File,
}

#[derive(Deserialize, Serialize, Clone)]
struct TestCase {
    note: Option<String>,
    args: Option<String>,
    input: Option<String>,
    out: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct TestSuite {
    tests: Vec<TestCase>,
}

struct TestRunner {
    program_path: std::path::PathBuf,
    test_cases: TestSuite,
}

impl TestRunner {
    pub fn new(program_path: &str, test_file: &str) -> Result<Self> {
        let program_path =
            std::fs::canonicalize(program_path).context("Failed to resolve program path")?;
        let test_file = std::fs::File::open(test_file).context("Failed to open test file")?;
        let test_cases = serde_yaml::from_reader::<_, TestSuite>(test_file)
            .context("Failed to parse test file")?;

        Ok(Self {
            program_path,
            test_cases,
        })
    }

    pub fn run(&self) -> Result<()> {
        for case in &self.test_cases.tests {
            self.run_test_case(case)?;
        }
        Ok(())
    }

    fn run_test_case(&self, case: &TestCase) -> Result<()> {
        let mut command = Command::new(&self.program_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args(case.args.as_deref().unwrap_or_default().split_whitespace())
            .spawn()
            .context("Failed to start program")?;

        if let Some(input) = &case.input {
            command
                .stdin
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?
                .write_all(input.as_bytes())
                .context("Failed to write input to program")?;
        }

        let output = command
            .wait_with_output()
            .context("Failed to get program output")?;

        let actual_output = String::from_utf8_lossy(&output.stdout);
        let expected_output = case.out.as_deref().unwrap_or_default();

        let diff = compute_diff(expected_output, &actual_output)?;

        println!("{}", case.note.as_deref().unwrap_or("Test case").bold());
        println!("{diff}");

        Ok(())
    }
}

fn read_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path).context(format!("Failed to read file: {path}"))
}

fn write_file(path: &Path, data: &str) -> Result<()> {
    std::fs::write(path, data).context(format!("Failed to write to file: {}", path.display()))
}

fn serialize_test_data(count: u8) -> Result<String> {
    let test = TestCase {
        note: Some("test".into()),
        args: Some("arguments".into()),
        input: Some("input".into()),
        out: Some("output".into()),
    };
    serde_yaml::to_string(&vec![test; count.into()])
        .context("Failed to serialize test data to YAML")
}

fn compute_diff(left: &str, right: &str) -> Result<DiffVec> {
    let dmp = DiffMatchPatch::new();
    dmp.diff_main::<Compat>(left, right)
        .map(DiffVec)
        .map_err(|e| anyhow::anyhow!("Diff computation failed: {e:?}"))
}

fn files_diff(left: &str, right: &str) -> Result<DiffVec> {
    compute_diff(&read_file(left)?, &read_file(right)?)
}

struct DiffVec(Vec<crate::dmp::Diff<char>>);

impl std::fmt::Display for DiffVec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for diff in &self.0 {
            let text = diff.data().iter().copied().collect::<String>();
            let colored_text = match diff.op() {
                Ops::Delete => text.on_red(),
                Ops::Equal => text.normal(),
                Ops::Insert => text.on_cyan(),
            };
            write!(f, "{colored_text}")?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Diff { left, right, mode } => match mode {
            Mode::Program => {
                TestRunner::new(&left, &right)?.run()?;
            }
            Mode::Interactive => println!("{}", compute_diff(&left, &right)?),
            Mode::File => println!("{}", files_diff(&left, &right)?),
        },
        Commands::Example { path, count } => {
            let data = serialize_test_data(count)?;
            if let Some(file) = path {
                write_file(&file, &data)?;
            } else {
                println!("{data}");
            }
        }
    }
    Ok(())
}
