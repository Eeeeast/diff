use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use diff_match_patch_rs::{Compat, DiffMatchPatch, Ops, dmp};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Get the diff between two inputs
    Get {
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
        count: u8,
        /// Output file path
        path: Option<std::path::PathBuf>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    /// Run the programme in the console to run the tests
    Program,
    /// Compare directly entered data
    Interactive,
    /// Compare a specified pair of files
    Batch,
}

#[derive(Deserialize, Serialize, Clone)]
struct Test {
    note: Option<String>,
    args: Option<String>,
    input: Option<String>,
    out: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Tests {
    tests: Vec<Test>,
}

struct Tester {
    app: std::path::PathBuf,
    tests: Vec<Test>,
}

impl Tester {
    pub fn new(program_path: &str, test_file: &str) -> Result<Self> {
        let app = fs::canonicalize(program_path).context("Failed to canonicalize program path")?;
        let content = fs::read_to_string(test_file).context("Failed to read test file")?;
        let tests = toml::from_str::<Tests>(&content)?.tests;
        Ok(Self { app, tests })
    }
}

fn read_file(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read file: {path}"))
}

fn write_file(path: &Path, data: &str) -> Result<()> {
    fs::write(path, data).with_context(|| format!("Failed to write to file: {}", path.display()))
}

fn serialize_test_data(count: u8) -> Result<String> {
    let test = Test {
        note: Some("test".into()),
        args: Some("arguments".into()),
        input: Some("input".into()),
        out: Some("output".into()),
    };
    toml::to_string(&vec![test; count.into()]).context("Failed to serialize test data")
}

fn diff(left: &str, right: &str) -> Result<DiffVec> {
    let dmp = DiffMatchPatch::new();
    dmp.diff_main::<Compat>(left, right)
        .map(DiffVec)
        .map_err(|e| anyhow::anyhow!("DiffMatchPatch error: {:?}", e))
}

fn files_diff(left: &str, right: &str) -> Result<DiffVec> {
    diff(&read_file(left)?, &read_file(right)?)
}

fn run_tests(tests: Vec<Test>, app: &Path) -> Result<()> {
    for test in tests {
        let mut child = Command::new(app)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args(test.args.unwrap_or_default().split_whitespace())
            .spawn()
            .context("Failed to spawn child process")?;

        if let Some(input) = test.input {
            child.stdin.as_mut().unwrap().write_all(input.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        let expected = test.out.unwrap_or_default();
        let diff_result = diff(&expected, &String::from_utf8_lossy(&output.stdout))?;
        println!("{}", test.note.unwrap_or_else(|| "test".into()));
        println!("{diff_result}");
    }
    Ok(())
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

    match cli.cmd {
        Commands::Get { left, right, mode } => match mode {
            Mode::Program => {
                let tester = Tester::new(&left, &right)?;
                run_tests(tester.tests, &tester.app)?;
            }
            Mode::Interactive => println!("{}", diff(&left, &right)?),
            Mode::Batch => println!("{}", files_diff(&left, &right)?),
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
