use clap::{Parser, ValueEnum};
use std::fmt::Debug;
use text_diff::print_diff;

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

fn main() {
    let mut cli: Cli = Cli::parse();
    println!("{:?}", cli);
    println!();
    loop {
        match cli.mode {
            Mode::Standart => cli.mode = Mode::Interactive,
            Mode::Program => {
                cli.mode = Mode::Interactive;
                break;
            }
            Mode::Interactive => {
                print_diff(&cli.left, &cli.right, " ");
                break;
            }
            Mode::Batch => {
                cli.mode = Mode::Interactive;
                break;
            }
        };
    }
}
