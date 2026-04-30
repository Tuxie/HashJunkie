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
        ("aich", "LMAGNHCIBVOP7PP2RPN2TFLBCYHS2G3X"),
        (
            "blake3",
            "882179b8dbccd285cda241d968cfcccb3156c5edac2fa3761bb6eda7ff8cb172",
        ),
        (
            "btv2",
            "785b0751fc2c53dc14a4ce3d800e69ef9ce1009eb327ccf458afe09c242c26c9",
        ),
        (
            "cidv0",
            "bafkreidylmdvd7bmkpobjjgohwaa42ppttqqbhvte7gpiwfp4cocilbgze",
        ),
        (
            "cidv1",
            "bafkreidylmdvd7bmkpobjjgohwaa42ppttqqbhvte7gpiwfp4cocilbgze",
        ),
        ("crc32", "b70b4c26"),
        (
            "dropbox",
            "05fe36f555179feb8712eadb2a1cadac8c3c7378859f8dbeaa8a6ea224ea3658",
        ),
        ("ed2k", "5ae257c47e9be1243ee32aabe408fb6b"),
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
        ("tiger", "4OQY25UN2XHIDQPV5U6BXAZ47INUCYGIBK7LFNI"),
        ("xxh128", "83885e853bb6640ca870f92984398d22"),
        ("xxh3", "a870f92984398d22"),
    ];

    assert!(!digests.contains_key(&Algorithm::Whirlpool));
    for (name, expected_hex) in expected {
        let alg: Algorithm = name.parse().unwrap();
        let got = digests
            .get(&alg)
            .unwrap_or_else(|| panic!("missing algorithm: {name}"));
        assert_eq!(got, expected_hex, "mismatch for {name}");
    }
}

#[test]
fn aich_matches_emule_tree_vectors_and_chunked_updates() {
    let cases: &[(&[u8], &str)] = &[
        (b"", "3I42H3S6NNFQ2MSVX7XZKYAYSCX5QBYJ"),
        (b"abc", "VGMT4NSHA2AWVOR6EVYXQUGCNSONBWE5"),
    ];

    for (data, expected) in cases {
        let mut h = MultiHasher::new(&[Algorithm::Aich]);
        h.update(data);
        assert_eq!(h.finalize()[&Algorithm::Aich], *expected);
    }

    let mut data = vec![0x11; 180 * 1024];
    data.push(0x22);

    let mut single = MultiHasher::new(&[Algorithm::Aich]);
    single.update(&data);
    assert_eq!(
        single.finalize()[&Algorithm::Aich],
        "J573AFG7KZF7FWRT4FS56AVF5EFGSV7B"
    );

    let mut chunked = MultiHasher::new(&[Algorithm::Aich]);
    for chunk in data.chunks(3333) {
        chunked.update(chunk);
    }
    assert_eq!(
        chunked.finalize()[&Algorithm::Aich],
        "J573AFG7KZF7FWRT4FS56AVF5EFGSV7B"
    );
}

#[test]
fn btv2_matches_bep52_pieces_root_vectors_and_chunked_updates() {
    let cases: &[(&[u8], &str)] = &[
        (
            b"",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
        (
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
    ];

    for (data, expected) in cases {
        let mut h = MultiHasher::new(&[Algorithm::Btv2]);
        h.update(data);
        assert_eq!(h.finalize()[&Algorithm::Btv2], *expected);
    }

    let mut data = vec![0x11; 16 * 1024];
    data.push(0x22);

    let mut single = MultiHasher::new(&[Algorithm::Btv2]);
    single.update(&data);
    assert_eq!(
        single.finalize()[&Algorithm::Btv2],
        "00fc3eb1148fae163d7387a6327f5c177693b8e548446cd3289b7614e2c136ac"
    );

    let mut chunked = MultiHasher::new(&[Algorithm::Btv2]);
    for chunk in data.chunks(777) {
        chunked.update(chunk);
    }
    assert_eq!(
        chunked.finalize()[&Algorithm::Btv2],
        "00fc3eb1148fae163d7387a6327f5c177693b8e548446cd3289b7614e2c136ac"
    );
}

#[test]
fn ed2k_matches_known_vectors_and_exact_block_boundary_rule() {
    let cases: &[(&[u8], &str)] = &[
        (b"", "31d6cfe0d16ae931b73c59d7e0c089c0"),
        (b"abc", "a448017aaf21d8525fc10ae87aa6729d"),
    ];

    for (data, expected) in cases {
        let mut h = MultiHasher::new(&[Algorithm::Ed2k]);
        h.update(data);
        assert_eq!(h.finalize()[&Algorithm::Ed2k], *expected);
    }

    let exact_block = vec![0xA5; 9_728_000];
    let mut h = MultiHasher::new(&[Algorithm::Ed2k]);
    h.update(&exact_block);
    assert_eq!(
        h.finalize()[&Algorithm::Ed2k],
        "9cab445c0310e326f5c73a1953882e84"
    );
}

#[test]
fn tiger_matches_known_empty_vector_and_chunked_updates() {
    let mut empty = MultiHasher::new(&[Algorithm::Tiger]);
    empty.update(b"");
    assert_eq!(
        empty.finalize()[&Algorithm::Tiger],
        "LWPNACQDBZRYXW3VHJVCJ64QBZNGHOHHHZWCLNQ"
    );

    let data = (0..2049).map(|i| (i % 251) as u8).collect::<Vec<_>>();
    let mut single = MultiHasher::new(&[Algorithm::Tiger]);
    single.update(&data);
    let single_digest = single.finalize()[&Algorithm::Tiger].clone();

    let mut chunked = MultiHasher::new(&[Algorithm::Tiger]);
    for chunk in data.chunks(333) {
        chunked.update(chunk);
    }

    assert_eq!(chunked.finalize()[&Algorithm::Tiger], single_digest);
}

#[test]
fn explicit_whirlpool_matches_known_vector_for_small_bin() {
    let data = std::fs::read("tests/fixtures/small.bin").expect("fixture file must exist");
    let mut h = MultiHasher::new(&[Algorithm::Whirlpool]);
    h.update(&data);
    let digests = h.finalize();
    assert_eq!(
        digests[&Algorithm::Whirlpool],
        "d606b7f44bd288759f8869d880d9d4a2f159d739005e72d00f93b814e8c04e657f40c838e4d6f9030a8c9e0308a4e3b450246250243b2f09e09fa5a24761e26b"
    );
}
