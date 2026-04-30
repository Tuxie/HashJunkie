use std::io::Cursor;

use hashjunkie::{Algorithm, hash_bytes, hash_file, hash_reader};

#[test]
fn hash_bytes_returns_standard_hex_and_raw_digests() {
    let result = hash_bytes(b"abc", &[Algorithm::Sha256, Algorithm::CidV1]);

    assert_eq!(
        result.standard(Algorithm::Sha256),
        Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
    assert_eq!(
        result.hex(Algorithm::CidV1).as_deref(),
        Some("01551220ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
    assert_eq!(
        result.raw(Algorithm::Sha256),
        Some(
            hex::decode("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
                .unwrap()
                .as_slice()
        )
    );
}

#[test]
fn hash_reader_uses_requested_algorithm_order_for_iteration() {
    let mut reader = Cursor::new(b"abc");
    let result = hash_reader(&mut reader, &[Algorithm::Sha512, Algorithm::Md5]).unwrap();

    let algorithms = result
        .iter()
        .map(|(algorithm, _)| algorithm)
        .collect::<Vec<_>>();
    assert_eq!(algorithms, vec![Algorithm::Sha512, Algorithm::Md5]);
}

#[test]
fn hash_file_hashes_files_without_cli_helpers() {
    let result = hash_file(
        "tests/fixtures/small.bin",
        &[Algorithm::Blake3, Algorithm::Sha1],
    )
    .unwrap();

    assert_eq!(
        result.standard(Algorithm::Blake3),
        Some("882179b8dbccd285cda241d968cfcccb3156c5edac2fa3761bb6eda7ff8cb172")
    );
    assert_eq!(
        result.standard(Algorithm::Sha1),
        Some("5b00669c480d5cffbdfa8bdba99561160f2d1b77")
    );
}
