use crate::dmp::Diff;
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
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Get the diff
    Get {
        /// What is being compared
        left: String,
        /// What's being compared to
        right: String,
        /// Compare mode
        #[clap(short, long, value_enum)]
        mode: Option<Mode>,
    },
    /// Get example of n I/O pairs to test
    Example {
        /// How many tests
        count: u16,
        /// Where to save
        path: Option<std::path::PathBuf>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
enum Mode {
    /// Run the program in the console to compare the given I/O value pairs
    /// Example of a file containing all optional variables
    /// [[tests]]
    /// note = "test"
    /// args = "arguments"
    /// input = "input"
    /// out = "output"
    #[clap(verbatim_doc_comment)]
    Program,
    /// Compare directly entered data
    Interactive,
    /// Compare a specified pair of files
    Batch,
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

enum DiffFilesError {
    Diff(diff_match_patch_rs::Error),
    LeftRead,
    RightRead,
    BothRead,
}

enum DiffProgramError {
    LeftRead,
    RightRead,
    RightParse(toml::de::Error),
    BothRead,
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

fn program_diff(left: &str, right: &str) -> Result<Tester, DiffProgramError> {
    let left = fs::canonicalize(left);
    let right = &fs::read_to_string(right);
    match left {
        Ok(app) => match right {
            Ok(r) => match toml::from_str::<Tests>(r) {
                Ok(tests) => Ok(Tester {
                    app,
                    tests: tests.tests,
                }),
                Err(e) => Err(DiffProgramError::RightParse(e)),
            },
            Err(_) => Err(DiffProgramError::RightRead),
        },
        Err(_) if right.is_err() => Err(DiffProgramError::BothRead),
        Err(_) => Err(DiffProgramError::LeftRead),
    }
}

fn diff(left: &str, right: &str) -> Result<DiffVec, diff_match_patch_rs::Error> {
    let dmp = DiffMatchPatch::new();
    Ok(DiffVec(dmp.diff_main::<Compat>(left, right)?))
}

fn files_diff(left: &str, right: &str) -> Result<DiffVec, DiffFilesError> {
    let left = &fs::read_to_string(left);
    let right = &fs::read_to_string(right);
    match left {
        Ok(l) => match right {
            Ok(r) => Ok(diff(l, r).map_err(DiffFilesError::Diff)?),
            Err(_) => Err(DiffFilesError::RightRead),
        },
        Err(_) if right.is_err() => Err(DiffFilesError::BothRead),
        Err(_) => Err(DiffFilesError::LeftRead),
    }
}

fn serialize_test_data(number: u16) -> Result<String, toml::ser::Error> {
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
}

fn main() {
    let cli = Cli::parse();
    println!("{:?}", cli);
    match cli.cmd {
        Commands::Get {
            left,
            right,
            mode: Some(Mode::Program),
        } => match program_diff(&left, &right) {
            Ok(o) => {
                for test in o.tests.into_iter() {
                    match &test.note {
                        Some(note) => println!("{}", note.on_green()),
                        None => println!("{}", "test!".on_green()),
                    }
                    let mut child = Command::new(&o.app)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .args(&test.args)
                        .spawn()
                        .expect("Failed to spawn child process");
                    if let Some(input) = test.input {
                        let mut stdin = child.stdin.take().expect("Failed to open stdin");
                        std::thread::spawn(move || {
                            stdin
                                .write_all(input.as_bytes())
                                .expect("Failed to write to stdin");
                        });
                    }
                    let output = child.wait_with_output().expect("Failed to read stdout");
                    match diff(
                        &test.out.unwrap_or("".to_string()),
                        &String::from_utf8_lossy(&output.stdout),
                    ) {
                        Ok(o) => println!("{}", o),
                        Err(e) => panic!("{:?}", e),
                    };
                }
            }
            Err(DiffProgramError::LeftRead) => {
                panic!("Failed to run this {}", Path::new(&left).display())
            }
            Err(DiffProgramError::RightRead) => panic!(
                "Could not read these {} I/O pairs",
                Path::new(&right).display()
            ),
            Err(DiffProgramError::RightParse(e)) => panic!(
                "Failed to deserialise this file {}. {:?}",
                Path::new(&right).display(),
                e
            ),
            Err(DiffProgramError::BothRead) => panic!(
                "Failed to read this {} program and these {} I/O pairs",
                Path::new(&left).display(),
                Path::new(&right).display()
            ),
        },
        Commands::Get {
            left,
            right,
            mode: Some(Mode::Interactive),
        } => match diff(&left, &right) {
            Ok(o) => println!("{}", o),
            Err(e) => panic!("{:?}", e),
        },
        Commands::Get {
            left,
            right,
            mode: Some(Mode::Batch),
        } => match files_diff(&left, &right) {
            Ok(o) => println!("{}", o),
            Err(DiffFilesError::Diff(e)) => panic!("{:?}", e),
            Err(DiffFilesError::LeftRead) => {
                panic!("Could not read this {} file", Path::new(&left).display())
            }
            Err(DiffFilesError::RightRead) => {
                panic!("Could not read this {} file", Path::new(&right).display())
            }
            Err(DiffFilesError::BothRead) => panic!(
                "Could not read these {}, {} files",
                Path::new(&left).display(),
                Path::new(&right).display()
            ),
        },
        Commands::Get {
            left,
            right,
            mode: None,
        } => match files_diff(&left, &right) {
            Ok(o) => println!("{}", o),
            Err(_) => match diff(&left, &right) {
                Ok(o) => println!("{}", o),
                Err(e) => panic!("{:?}", e),
            },
        },
        Commands::Example { path, count } => {
            let data = serialize_test_data(count).expect("Serialisation error unexpected");
            if let Some(file) = path {
                match fs::File::create_new(file.clone()) {
                    Ok(mut file) => file.write_all(data.as_bytes()).unwrap(),
                    Err(e) => panic!(
                        "This {} file already exists. Only creating a new file is allowed. {}",
                        file.display(),
                        e
                    ),
                }
            } else {
                println!("{}", data)
            }
        }
    }
}
