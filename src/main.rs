use clap::{Parser, ValueEnum};
use colored::Colorize;
use core::str;
use diff_match_patch_rs::*;
use serde::Deserialize;
use std::fmt::Debug;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// What is being compared
    left: String,
    /// What's being compared to
    right: String,
    #[clap(short, long, value_enum, default_value_t=Mode::Standart)]
    mode: Mode,
}

#[derive(ValueEnum, Clone, PartialEq, Debug)]
enum Mode {
    /// Automatic comparison based on specified rules
    Standart,
    /// Executing the specified console program to compare the provided I/O value pairs
    /// Expected toml file and application path
    /// Example toml file with all arguments optional:
    /// [[tests]]
    /// note = "test 1"
    /// arguments = "arguments"
    /// input = "input"
    /// out = "output"
    #[clap(verbatim_doc_comment)]
    Program,
    /// Comparison of directly entered data
    Interactive,
    /// Comparing a specified pair of files
    Batch,
}

#[derive(Deserialize, Debug)]
struct Config {
    tests: Vec<Test>,
}

#[derive(Deserialize, Debug)]
struct Test {
    note: Option<String>,
    arguments: Option<String>,
    input: Option<String>,
    out: Option<String>,
}

fn diff(left: &str, right: &str) {
    let dmp = DiffMatchPatch::new();
    match dmp.diff_main::<Compat>(left, right) {
        Ok(diffs) => {
            for diff in diffs {
                print!(
                    "{}",
                    match diff.op() {
                        Ops::Delete => diff.data().iter().copied().collect::<String>().on_red(),
                        Ops::Equal => diff.data().iter().copied().collect::<String>().normal(),
                        Ops::Insert => diff.data().iter().copied().collect::<String>().on_cyan(),
                    }
                )
            }
        }
        Err(e) => panic!("{:?}", e),
    }
}

fn main() {
    let mut cli: Cli = Cli::parse();
    println!("{:?}", cli);
    println!();
    loop {
        match cli.mode {
            Mode::Standart => {
                if Path::new(&cli.left).is_file() && Path::new(&cli.right).is_file() {
                    if Path::new(&cli.left)
                        .extension()
                        .is_some_and(|n| n.eq("toml"))
                    {
                        cli.mode = Mode::Program;
                    } else {
                        cli.mode = Mode::Batch;
                    }
                } else {
                    cli.mode = Mode::Interactive;
                }
            }
            Mode::Program => {
                let tests = Path::new(&cli.left);
                let config: Config = match tests.extension().and_then(std::ffi::OsStr::to_str) {
                    Some("toml") => {
                        toml::from_str(&fs::read_to_string(&cli.left).unwrap_or_else(|_| {
                            panic!("Was supposed to read the {:?} file", Path::new(&cli.left))
                        }))
                        .unwrap_or_else(|_| {
                            panic!("Failed to read .toml file {:?}", Path::new(&cli.left));
                        })
                    }
                    _ => panic!("Unexpected {:?} file with tests", tests),
                };
                for test in config.tests {
                    println!("{}", &test.note.unwrap_or("".to_string()));
                    let mut child = Command::new(&cli.right)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .args(test.arguments)
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
                    diff(
                        &test.out.unwrap_or("".to_string()),
                        &String::from_utf8_lossy(&output.stdout),
                    );
                    println!();
                }
                break;
            }
            Mode::Interactive => {
                diff(&cli.left, &cli.right);
                break;
            }
            Mode::Batch => {
                diff(
                    &fs::read_to_string(&cli.left).unwrap_or_else(|_| {
                        panic!("Was supposed to read the {:?} file", Path::new(&cli.left))
                    }),
                    &fs::read_to_string(&cli.right).unwrap_or_else(|_| {
                        panic!("Was supposed to read the {:?} file", Path::new(&cli.right))
                    }),
                );
                break;
            }
        };
    }
}
