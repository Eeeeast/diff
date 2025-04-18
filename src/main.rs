use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use diff_match_patch_rs::{Compat, DiffMatchPatch, Ops, dmp};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
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
    Example,
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

const STYLE_RED: anstyle::Style =
    anstyle::Style::new().bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red)));
const STYLE_GREEN: anstyle::Style =
    anstyle::Style::new().bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
const STYLE_CYAN: anstyle::Style =
    anstyle::Style::new().bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)));

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

        let expected_output = case.out.as_deref().unwrap_or_default();
        let actual_output = String::from_utf8_lossy(&output.stdout);

        if expected_output == actual_output {
            println!(
                "{STYLE_GREEN}{}{STYLE_GREEN:#}\n{actual_output}",
                case.note.as_deref().unwrap_or("Unnamed test case")
            );
        } else {
            let diff = compute_diff(expected_output, &actual_output)?;

            println!(
                "{STYLE_RED}{}{STYLE_RED:#}\n{diff}",
                case.note.as_deref().unwrap_or("Unnamed test case")
            );
        }

        Ok(())
    }
}

fn read_file(path: &str) -> Result<String> {
    std::fs::read_to_string(path).context(format!("Failed to read file: {path}"))
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
            match diff.op() {
                Ops::Delete => write!(f, "{STYLE_RED}{text}{STYLE_RED:#}"),
                Ops::Equal => write!(f, "{text}"),
                Ops::Insert => write!(f, "{STYLE_CYAN}{text}{STYLE_CYAN:#}"),
            }?;
        }
        Ok(())
    }
}

const EXAMPLE_STRING: &str = r"
 - note: test
   args: arguments
   input: input
   out: output
 - note: test
   args: arguments
   input: input
   out: output
 - note: test
   args: arguments
   input: input
   out: output
";

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
        Commands::Example => {
            println!("{EXAMPLE_STRING}");
        }
    }
    Ok(())
}
