mod args;
mod output;

use args::{Args, Format};
use clap::Parser;
use hashjunkie_core::{Algorithm, MultiHasher};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read};
use std::process;

const CHUNK_SIZE: usize = 64 * 1024;

/// Streams all bytes from `reader` through the given algorithms in 64 KiB chunks.
/// Returns a sorted map of algorithm name → lowercase hex digest.
fn hash_reader<R: Read>(
    reader: &mut R,
    algorithms: &[Algorithm],
) -> io::Result<BTreeMap<String, String>> {
    let mut hasher = MultiHasher::new(algorithms);
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let mut sorted = BTreeMap::new();
    for (alg, hex) in hasher.finalize() {
        sorted.insert(alg.as_str().to_string(), hex);
    }
    Ok(sorted)
}

fn run_stdin(algorithms: &[Algorithm], format: &Format) -> i32 {
    match hash_reader(&mut io::stdin(), algorithms) {
        Ok(digests) => {
            let out = match format {
                Format::Json => output::format_as_json_object(&digests),
                Format::Hex => output::format_as_hex_lines(&digests),
            };
            println!("{}", out.trim_end_matches('\n'));
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn run_files(algorithms: &[Algorithm], files: &[String], format: &Format) -> i32 {
    let mut results: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    let mut exit_code = 0;

    for path in files {
        match File::open(path) {
            Ok(mut f) => match hash_reader(&mut f, algorithms) {
                Ok(digests) => results.push((path.clone(), digests)),
                Err(e) => {
                    eprintln!("{path}: {e}");
                    exit_code = 1;
                }
            },
            Err(e) => {
                eprintln!("{path}: {e}");
                exit_code = 1;
            }
        }
    }

    if !results.is_empty() {
        let pairs: Vec<(&str, &BTreeMap<String, String>)> =
            results.iter().map(|(p, d)| (p.as_str(), d)).collect();
        let out = match format {
            Format::Json => output::format_as_file_json(&pairs),
            Format::Hex => output::format_as_file_hex(&pairs),
        };
        println!("{}", out.trim_end_matches('\n'));
    }

    exit_code
}

fn main() {
    let args = Args::parse();

    let algorithms = match args.resolved_algorithms() {
        Ok(algs) => algs,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let code = if args.files.is_empty() {
        run_stdin(&algorithms, &args.format)
    } else {
        run_files(&algorithms, &args.files, &args.format)
    };

    process::exit(code);
}
