use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use diff_match_patch_rs::*;
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
        /// Left input (file or string)
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
        count: u16,
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
        Ok(Self {
            app,
            tests: toml::from_str::<Tests>(&content)
                .context("Failed to parse TOML file")?
                .tests,
        })
    }
}

fn read_file(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))
}

fn write_file(path: &Path, data: &str) -> Result<()> {
    fs::write(path, data).with_context(|| format!("Failed to write to file: {}", path.display()))
}

fn serialize_test_data(count: u16) -> Result<String> {
    toml::to_string(&Tests {
        tests: vec![
            Test {
                note: Some("test".to_string()),
                args: Some("arguments".to_string()),
                input: Some("input".to_string()),
                out: Some("output".to_string()),
            };
            count.into()
        ],
    })
    .context("Failed to serialize test data")
}

#[derive(Debug)]
struct DiffMatchPatchWrapper(diff_match_patch_rs::Error);

impl std::fmt::Display for DiffMatchPatchWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DiffMatchPatch Error: {:?}", self.0)
    }
}

impl std::error::Error for DiffMatchPatchWrapper {}

fn diff(left: &str, right: &str) -> Result<DiffVec> {
    let dmp = DiffMatchPatch::new();
    dmp.diff_main::<Compat>(left, right)
        .map(DiffVec)
        .map_err(|e| anyhow::Error::new(DiffMatchPatchWrapper(e)).context("DiffMatchPatch error"))
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
        println!("{}", test.note.unwrap_or_else(|| "test".to_string()));
        println!("{}", diff_result);
    }
    Ok(())
}

struct DiffVec(Vec<crate::dmp::Diff<char>>);

impl std::fmt::Display for DiffVec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for diff in &self.0 {
            let text = diff.data().iter().copied().collect::<String>();
            write!(
                f,
                "{}",
                match diff.op() {
                    Ops::Delete => text.on_red(),
                    Ops::Equal => text.normal(),
                    Ops::Insert => text.on_cyan(),
                }
            )?;
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
                run_tests(tester.tests, &tester.app)?
            }
            Mode::Interactive => println!("{}", diff(&left, &right)?),
            Mode::Batch => println!("{}", files_diff(&left, &right)?),
        },
        Commands::Example { path, count } => {
            let data = serialize_test_data(count)?;
            if let Some(file) = path {
                write_file(&file, &data)?;
            } else {
                println!("{}", data);
            }
        }
    }
    Ok(())
}
