mod args;
mod output;

use args::{Args, Format};
use chrono::{SecondsFormat, Utc};
use clap::Parser;
use hashjunkie_core::{Algorithm, MultiHasher, PipelinedHashError, PipelinedMultiHasher};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::process;
use std::time::SystemTime;

const CHUNK_SIZE: usize = 1024 * 1024;
const _: () = assert!(CHUNK_SIZE >= 128 * 1024);

/// Streams all bytes from `reader` through the given algorithms in large chunks.
/// Returns a sorted map of algorithm name to digest string.
fn hash_reader<R: Read>(
    reader: &mut R,
    algorithms: &[Algorithm],
) -> io::Result<BTreeMap<String, String>> {
    if algorithms.len() > 1 {
        return hash_reader_pipelined(reader, algorithms);
    }

    let mut hasher = MultiHasher::new(algorithms);
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update_parallel(&buf[..n]);
    }
    let mut sorted = BTreeMap::new();
    for (alg, hex) in hasher.finalize() {
        sorted.insert(alg.as_str().to_string(), hex);
    }
    Ok(sorted)
}

fn pipeline_error(err: PipelinedHashError) -> io::Error {
    io::Error::other(err)
}

fn hash_reader_pipelined<R: Read>(
    reader: &mut R,
    algorithms: &[Algorithm],
) -> io::Result<BTreeMap<String, String>> {
    let mut hasher = PipelinedMultiHasher::new(algorithms);
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]).map_err(pipeline_error)?;
    }

    let mut sorted = BTreeMap::new();
    for (alg, hex) in hasher.finalize().map_err(pipeline_error)? {
        sorted.insert(alg.as_str().to_string(), hex);
    }
    Ok(sorted)
}

struct CountingReader<R> {
    inner: R,
    bytes_read: u64,
}

impl<R> CountingReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            bytes_read: 0,
        }
    }
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.bytes_read += n as u64;
        Ok(n)
    }
}

fn format_system_time(time: SystemTime) -> String {
    chrono::DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn file_name_for_path(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

/// Hash any `Read` source and print the result. Extracted from `run_stdin` for testability.
fn run_reader<R: Read>(
    reader: &mut R,
    algorithms: &[Algorithm],
    format: &Format,
    hashes_only: bool,
) -> i32 {
    match hash_reader(reader, algorithms) {
        Ok(digests) => {
            let out = if hashes_only {
                output::format_as_hashes_only(algorithms, &digests)
            } else {
                match format {
                    Format::Json => output::format_as_json_object(&digests),
                    Format::Hex => output::format_as_hex_lines(&digests),
                    Format::Line => output::format_as_file_line(algorithms, "-", 0, &digests),
                }
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

fn run_stdin(algorithms: &[Algorithm], format: &Format, hashes_only: bool) -> i32 {
    if hashes_only {
        return run_reader(&mut io::stdin(), algorithms, format, true);
    }

    match format {
        Format::Hex => run_reader(&mut io::stdin(), algorithms, format, false),
        Format::Json => {
            let mut reader = CountingReader::new(io::stdin());
            match hash_reader(&mut reader, algorithms) {
                Ok(digests) => {
                    let entry = output::FileJsonEntry {
                        path: "-",
                        name: "-",
                        size: reader.bytes_read,
                        mod_time: format_system_time(SystemTime::now()),
                        digests: &digests,
                    };
                    println!("{}", output::format_as_stdin_json(&entry));
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        Format::Line => {
            let mut reader = CountingReader::new(io::stdin());
            match hash_reader(&mut reader, algorithms) {
                Ok(digests) => {
                    println!(
                        "{}",
                        output::format_as_file_line(algorithms, "-", reader.bytes_read, &digests)
                    );
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
    }
}

fn run_files(
    algorithms: &[Algorithm],
    files: &[String],
    format: &Format,
    hashes_only: bool,
) -> i32 {
    let mut results: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    let mut json_metadata: Vec<(String, u64, String)> = Vec::new();
    let mut sizes: Vec<u64> = Vec::new();
    let mut exit_code = 0;

    for path in files.iter().take(if hashes_only { 1 } else { files.len() }) {
        match std::fs::metadata(path) {
            Ok(metadata) => match metadata.modified() {
                Ok(modified) => match File::open(path) {
                    Ok(mut f) => match hash_reader(&mut f, algorithms) {
                        Ok(digests) => {
                            if matches!(format, Format::Json) {
                                json_metadata.push((
                                    file_name_for_path(path).to_string(),
                                    metadata.len(),
                                    format_system_time(modified),
                                ));
                            }
                            sizes.push(metadata.len());
                            results.push((path.clone(), digests));
                        }
                        Err(e) => {
                            eprintln!("{path}: {e}");
                            exit_code = 1;
                        }
                    },
                    Err(e) => {
                        eprintln!("{path}: {e}");
                        exit_code = 1;
                    }
                },
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
        if hashes_only {
            println!(
                "{}",
                output::format_as_hashes_only(algorithms, &results[0].1).trim_end_matches('\n')
            );
            return exit_code;
        }

        let pairs: Vec<(&str, &BTreeMap<String, String>)> = results
            .iter()
            .map(|(path, digests)| (path.as_str(), digests))
            .collect();
        let out = match format {
            Format::Json => {
                let entries: Vec<output::FileJsonEntry<'_>> = results
                    .iter()
                    .zip(json_metadata.iter())
                    .map(
                        |((path, digests), (name, size, mod_time))| output::FileJsonEntry {
                            path,
                            name,
                            size: *size,
                            mod_time: mod_time.clone(),
                            digests,
                        },
                    )
                    .collect();
                output::format_as_file_json(&entries)
            }
            Format::Hex => output::format_as_file_hex(&pairs),
            Format::Line => {
                let entries: Vec<output::FileLineEntry<'_>> = results
                    .iter()
                    .zip(sizes.iter())
                    .map(|((path, digests), size)| output::FileLineEntry {
                        path,
                        size: *size,
                        digests,
                    })
                    .collect();
                output::format_as_file_lines(&entries, algorithms)
            }
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
        run_stdin(&algorithms, &args.format, args.hashes_only)
    } else {
        run_files(&algorithms, &args.files, &args.format, args.hashes_only)
    };

    process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Read` impl that always returns an IO error — exercises the error branch
    /// of `hash_reader` and `run_reader` without requiring real I/O failures.
    struct ErrorReader;
    impl io::Read for ErrorReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected read error"))
        }
    }

    #[test]
    fn hash_reader_propagates_io_error() {
        let algs = [Algorithm::Sha256];
        let result = hash_reader(&mut ErrorReader, &algs);
        assert!(result.is_err());
    }

    #[test]
    fn pipelined_hash_reader_matches_single_update() {
        let data = vec![17; 1024 * 1024 + 13];
        let algs = [
            Algorithm::Blake3,
            Algorithm::Sha256,
            Algorithm::Sha512,
            Algorithm::Dropbox,
        ];

        let pipelined = hash_reader(&mut data.as_slice(), &algs).unwrap();

        let mut hasher = MultiHasher::new(&algs);
        hasher.update(&data);
        let expected = hasher
            .finalize()
            .into_iter()
            .map(|(alg, digest)| (alg.as_str().to_string(), digest))
            .collect();

        assert_eq!(pipelined, expected);
    }

    #[test]
    fn run_reader_returns_1_on_read_error() {
        let algs = [Algorithm::Sha256];
        let code = run_reader(&mut ErrorReader, &algs, &Format::Json, false);
        assert_eq!(code, 1);
    }
}
