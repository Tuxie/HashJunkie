use hashjunkie_core::{Algorithm, MultiHasher};

fn hash_file(path: &str) -> std::collections::HashMap<Algorithm, String> {
    let data = std::fs::read(path).expect("fixture file must exist");
    let mut h = MultiHasher::all();
    h.update(&data);
    h.finalize()
}

/// Ground truth for tests/fixtures/small.bin (1 KiB, bytes 0x00..0xFF repeated 4×)
/// sha256, sha1, md5 verified against system tools (sha256sum, sha1sum, md5sum).
#[test]
fn all_algorithms_match_known_vectors_for_small_bin() {
    let digests = hash_file("tests/fixtures/small.bin");

    let expected: &[(&str, &str)] = &[
        (
            "blake3",
            "882179b8dbccd285cda241d968cfcccb3156c5edac2fa3761bb6eda7ff8cb172",
        ),
        ("crc32", "b70b4c26"),
        (
            "dropbox",
            "05fe36f555179feb8712eadb2a1cadac8c3c7378859f8dbeaa8a6ea224ea3658",
        ),
        ("hidrive", "5b00669c480d5cffbdfa8bdba99561160f2d1b77"),
        ("mailru", "2b4639914e8e0e8f99d2a90a23801c7a87a089c1"),
        ("md5", "b2ea9f7fcea831a4a63b213f41a8855b"),
        ("quickxor", "87b86bd9d6c26b264241847d28ac65c03b93e142"),
        ("sha1", "5b00669c480d5cffbdfa8bdba99561160f2d1b77"),
        (
            "sha256",
            "785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9",
        ),
        (
            "sha512",
            "37f652be867f28ed033269cbba201af2112c2b3fd334a89fd2f757938ddee815787cc61d6e24a8a33340d0f7e86ffc058816b88530766ba6e231620a130b566c",
        ),
        (
            "whirlpool",
            "d606b7f44bd288759f8869d880d9d4a2f159d739005e72d00f93b814e8c04e657f40c838e4d6f9030a8c9e0308a4e3b450246250243b2f09e09fa5a24761e26b",
        ),
        ("xxh128", "83885e853bb6640ca870f92984398d22"),
        ("xxh3", "a870f92984398d22"),
    ];

    for (name, expected_hex) in expected {
        let alg: Algorithm = name.parse().unwrap();
        let got = digests
            .get(&alg)
            .unwrap_or_else(|| panic!("missing algorithm: {name}"));
        assert_eq!(got, expected_hex, "mismatch for {name}");
    }
}
