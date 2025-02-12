use crate::dmp::Diff;
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use diff_match_patch_rs::*;
use serde::{Deserialize, Serialize};
use std::fmt;
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
    /// Constructs a new `Tester` instance by reading the program path and test cases from a TOML file.
    ///
    /// # Arguments
    /// * `program_path` - The path to the executable program.
    /// * `test_file` - The path to the TOML file containing test cases.
    ///
    /// # Returns
    /// A `Result` containing the `Tester` instance or an error if something goes wrong.
    pub fn new(program_path: &str, test_file: &str) -> Result<Self> {
        // Canonicalize the program path
        let app = fs::canonicalize(program_path).context("Failed to canonicalize program path")?;

        // Read and parse the TOML file containing test cases
        let content = fs::read_to_string(test_file).context("Failed to read test file")?;
        let tests: Tests = toml::from_str(&content).context("Failed to parse TOML file")?;

        Ok(Self {
            app,
            tests: tests.tests,
        })
    }
}

fn read_file(path: &str) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))
}

fn write_file(path: &Path, data: &str) -> Result<()> {
    fs::write(path, data).with_context(|| format!("Failed to write to file: {}", path.display()))
}

fn serialize_test_data(number: u16) -> Result<String> {
    toml::to_string::<Tests>(&Tests {
        tests: vec![
            Test {
                note: Some("test".to_string()),
                args: Some("arguments".to_string()),
                input: Some("input".to_string()),
                out: Some("output".to_string()),
            };
            number.into()
        ],
    })
    .context("Failed to serialize test data")
}

#[derive(Debug)]
struct DiffMatchPatchError(diff_match_patch_rs::Error);

impl fmt::Display for DiffMatchPatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DiffMatchPatch Error: {:?}", self.0)
    }
}

impl std::error::Error for DiffMatchPatchError {}

fn diff(left: &str, right: &str) -> Result<DiffVec> {
    let dmp = DiffMatchPatch::new();
    let result = dmp
        .diff_main::<Compat>(left, right)
        .map_err(DiffMatchPatchError)?;
    Ok(DiffVec(result))
}

fn files_diff(left: &str, right: &str) -> Result<DiffVec> {
    let left_content = read_file(left)?;
    let right_content = read_file(right)?;
    diff(&left_content, &right_content)
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
            let mut stdin = child.stdin.take().context("Failed to open stdin")?;
            stdin
                .write_all(input.as_bytes())
                .context("Failed to write to stdin")?;
        }

        let output = child.wait_with_output().context("Failed to read stdout")?;
        let expected = test.out.unwrap_or_default();
        let diff_result = diff(&expected, &String::from_utf8_lossy(&output.stdout))?;
        if let Some(note) = test.note {
            println!("{}", note);
        } else {
            println!("test");
        }
        println!("{}", diff_result);
    }
    Ok(())
}

struct DiffVec(Vec<Diff<char>>);

impl fmt::Display for DiffVec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for diff in self.0.iter() {
            write!(
                f,
                "{}",
                match diff.op() {
                    Ops::Delete => diff.data().iter().copied().collect::<String>().on_red(),
                    Ops::Equal => diff.data().iter().copied().collect::<String>().normal(),
                    Ops::Insert => diff.data().iter().copied().collect::<String>().on_cyan(),
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
                run_tests(tester.tests, &tester.app)?;
            }
            Mode::Interactive => {
                let result = diff(&left, &right)?;
                println!("{}", result);
            }
            Mode::Batch => {
                let result = files_diff(&left, &right)?;
                println!("{}", result);
            }
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
