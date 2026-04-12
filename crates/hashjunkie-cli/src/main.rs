mod args;

use args::Args;
use clap::Parser;
use std::process;

fn main() {
    let args = Args::parse();

    let algorithms = match args.resolved_algorithms() {
        Ok(algs) => algs,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let _ = algorithms; // used in subsequent tasks
}
