use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

use hashjunkie_core::hashes::{CidHasher, CidProfile, Hasher, reset_profile, take_profile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: hashjunkie-cid-profile [cidv0|cidv1] PATH");
        std::process::exit(2);
    };
    let Some(file_path) = args.next() else {
        eprintln!("Usage: hashjunkie-cid-profile [cidv0|cidv1] PATH");
        std::process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("Usage: hashjunkie-cid-profile [cidv0|cidv1] PATH");
        std::process::exit(2);
    }

    let mut hasher: Box<dyn Hasher> = match path.as_str() {
        "cidv0" => Box::new(CidHasher::v0()),
        "cidv1" => Box::new(CidHasher::v1()),
        other => {
            eprintln!("unknown CID version: {other}");
            std::process::exit(2);
        }
    };

    let mut file = File::open(&file_path)?;
    let mut buffer = vec![0; 1024 * 1024];
    let mut bytes = 0_u64;

    reset_profile();
    let total_started = Instant::now();
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        hasher.update(&buffer[..read]);
    }
    let cid = hasher.finalize_hex();
    let total = total_started.elapsed();
    let profile = take_profile();

    println!("file: {file_path}");
    println!("algorithm: {path}");
    println!("bytes: {bytes}");
    println!("cid: {cid}");
    println!("total_ms: {:.3}", total.as_secs_f64() * 1000.0);
    print_profile(profile);

    Ok(())
}

fn print_profile(profile: CidProfile) {
    let measured_ns = profile.chunk_buffering_ns
        + profile.raw_leaf_hashing_ns
        + profile.dag_pb_encoding_ns
        + profile.dag_pb_hashing_ns
        + profile.cid_text_encoding_ns;

    println!();
    println!("{:<24} {:>12} {:>8}", "phase", "ms", "measured");
    print_phase("chunk_buffering", profile.chunk_buffering_ns, measured_ns);
    print_phase("raw_leaf_hashing", profile.raw_leaf_hashing_ns, measured_ns);
    print_phase("dag_pb_encoding", profile.dag_pb_encoding_ns, measured_ns);
    print_phase("dag_pb_hashing", profile.dag_pb_hashing_ns, measured_ns);
    print_phase(
        "cid_text_encoding",
        profile.cid_text_encoding_ns,
        measured_ns,
    );
    print_phase("measured_total", measured_ns, measured_ns);
}

fn print_phase(label: &str, ns: u64, measured_ns: u64) {
    let ms = ns as f64 / 1_000_000.0;
    let pct = if measured_ns == 0 {
        0.0
    } else {
        ns as f64 * 100.0 / measured_ns as f64
    };
    println!("{label:<24} {ms:>12.3} {pct:>7.2}%");
}
