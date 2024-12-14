use clap::{Parser, ValueEnum};
use colored::Colorize;
use core::str;
use diff_match_patch_rs::*;
use std::fmt::Debug;
use std::fs;
use std::path::Path;

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
    Program,
    /// Comparison of directly entered data
    Interactive,
    /// Comparing a specified pair of files
    Batch,
}

fn diff(left: &str, right: &str) {
    let dmp = DiffMatchPatch::new();
    match dmp.diff_main::<Compat>(left, right) {
        Ok(diffs) => {
            for diff in diffs {
                match diff.op() {
                    Ops::Delete => {
                        print!("{}", diff.data().iter().copied().collect::<String>().red())
                    }
                    Ops::Equal => {
                        print!("{}", diff.data().iter().copied().collect::<String>())
                    }
                    Ops::Insert => {
                        print!(
                            "{}",
                            diff.data().iter().copied().collect::<String>().green()
                        )
                    }
                }
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
                    cli.mode = Mode::Batch;
                } else {
                    cli.mode = Mode::Interactive;
                }
            }
            Mode::Program => {
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
