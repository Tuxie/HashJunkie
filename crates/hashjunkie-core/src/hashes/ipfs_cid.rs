use digest::Digest;
use rayon::prelude::*;
use sha2::Sha256;
#[cfg(any(feature = "profile-ipfs-cid", test))]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(any(feature = "profile-ipfs-cid", test))]
use std::time::{Duration, Instant};

use crate::{DigestValue, base32_lower_no_padding_multibase};

use super::Hasher;

const CHUNK_SIZE: usize = 262_144;
const MAX_LINKS: usize = 174;
const PARALLEL_CHUNK_BATCH_SIZE: usize = 64;
const CID_VERSION: u64 = 1;
const MULTICODEC_RAW: u64 = 0x55;
const MULTICODEC_DAG_PB: u64 = 0x70;
const MULTIHASH_SHA2_256: u64 = 0x12;
const UNIXFS_FILE_TYPE: u64 = 2;
const BASE58BTC_ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Debug, Clone)]
struct UnixFsBlock {
    cid: Vec<u8>,
    multihash: Vec<u8>,
    codec: u64,
    file_size: u64,
    tsize: u64,
}

#[derive(Debug, Clone, Copy)]
enum CidVersion {
    V0,
    V1,
}

pub struct CidHasher {
    current: Vec<u8>,
    pending_chunks: Vec<Vec<u8>>,
    leaves: Vec<UnixFsBlock>,
    version: CidVersion,
}

#[cfg(any(feature = "profile-ipfs-cid", test))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CidProfile {
    pub chunk_buffering_ns: u64,
    pub raw_leaf_hashing_ns: u64,
    pub dag_pb_encoding_ns: u64,
    pub dag_pb_hashing_ns: u64,
    pub cid_text_encoding_ns: u64,
}

#[cfg(any(feature = "profile-ipfs-cid", test))]
#[derive(Clone, Copy)]
enum ProfilePhase {
    ChunkBuffering,
    RawLeafHashing,
    DagPbEncoding,
    DagPbHashing,
    CidTextEncoding,
}

#[cfg(any(feature = "profile-ipfs-cid", test))]
static CHUNK_BUFFERING_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(any(feature = "profile-ipfs-cid", test))]
static RAW_LEAF_HASHING_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(any(feature = "profile-ipfs-cid", test))]
static DAG_PB_ENCODING_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(any(feature = "profile-ipfs-cid", test))]
static DAG_PB_HASHING_NS: AtomicU64 = AtomicU64::new(0);
#[cfg(any(feature = "profile-ipfs-cid", test))]
static CID_TEXT_ENCODING_NS: AtomicU64 = AtomicU64::new(0);

#[cfg(any(feature = "profile-ipfs-cid", test))]
fn add_duration(counter: &AtomicU64, elapsed: Duration) {
    let nanos = elapsed.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(nanos, Ordering::Relaxed);
}

#[cfg(any(feature = "profile-ipfs-cid", test))]
fn record_profile(phase: ProfilePhase, started: Instant) {
    match phase {
        ProfilePhase::ChunkBuffering => add_duration(&CHUNK_BUFFERING_NS, started.elapsed()),
        ProfilePhase::RawLeafHashing => add_duration(&RAW_LEAF_HASHING_NS, started.elapsed()),
        ProfilePhase::DagPbEncoding => add_duration(&DAG_PB_ENCODING_NS, started.elapsed()),
        ProfilePhase::DagPbHashing => add_duration(&DAG_PB_HASHING_NS, started.elapsed()),
        ProfilePhase::CidTextEncoding => add_duration(&CID_TEXT_ENCODING_NS, started.elapsed()),
    }
}

#[cfg(any(feature = "profile-ipfs-cid", test))]
pub fn reset_profile() {
    CHUNK_BUFFERING_NS.store(0, Ordering::Relaxed);
    RAW_LEAF_HASHING_NS.store(0, Ordering::Relaxed);
    DAG_PB_ENCODING_NS.store(0, Ordering::Relaxed);
    DAG_PB_HASHING_NS.store(0, Ordering::Relaxed);
    CID_TEXT_ENCODING_NS.store(0, Ordering::Relaxed);
}

#[cfg(any(feature = "profile-ipfs-cid", test))]
pub fn take_profile() -> CidProfile {
    CidProfile {
        chunk_buffering_ns: CHUNK_BUFFERING_NS.load(Ordering::Relaxed),
        raw_leaf_hashing_ns: RAW_LEAF_HASHING_NS.load(Ordering::Relaxed),
        dag_pb_encoding_ns: DAG_PB_ENCODING_NS.load(Ordering::Relaxed),
        dag_pb_hashing_ns: DAG_PB_HASHING_NS.load(Ordering::Relaxed),
        cid_text_encoding_ns: CID_TEXT_ENCODING_NS.load(Ordering::Relaxed),
    }
}

impl CidHasher {
    pub fn v0() -> Self {
        Self::new(CidVersion::V0)
    }

    pub fn v1() -> Self {
        Self::new(CidVersion::V1)
    }

    fn new(version: CidVersion) -> Self {
        Self {
            current: Vec::new(),
            pending_chunks: Vec::new(),
            leaves: Vec::new(),
            version,
        }
    }

    fn push_chunk(&mut self, chunk: &[u8]) {
        self.pending_chunks.push(chunk.to_vec());
    }

    fn push_owned_chunk(&mut self, chunk: Vec<u8>) {
        self.pending_chunks.push(chunk);
        if self.pending_chunks.len() >= PARALLEL_CHUNK_BATCH_SIZE {
            self.flush_pending_chunks();
        }
    }

    fn flush_pending_chunks(&mut self) {
        let pending = std::mem::take(&mut self.pending_chunks);
        self.leaves.extend(
            pending
                .into_par_iter()
                .map(|chunk| raw_block(&chunk))
                .collect::<Vec<_>>(),
        );
    }
}

impl Hasher for CidHasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let remaining = CHUNK_SIZE - self.current.len();
            let take = remaining.min(data.len());
            #[cfg(any(feature = "profile-ipfs-cid", test))]
            let started = Instant::now();
            self.current.extend_from_slice(&data[..take]);
            #[cfg(any(feature = "profile-ipfs-cid", test))]
            record_profile(ProfilePhase::ChunkBuffering, started);
            data = &data[take..];

            if self.current.len() == CHUNK_SIZE {
                let chunk = std::mem::take(&mut self.current);
                self.push_owned_chunk(chunk);
            }
        }
    }

    fn finalize_hex(self: Box<Self>) -> String {
        self.finalize_digest().standard().to_string()
    }

    fn finalize_digest(mut self: Box<Self>) -> DigestValue {
        if !self.current.is_empty() || (self.leaves.is_empty() && self.pending_chunks.is_empty()) {
            let chunk = std::mem::take(&mut self.current);
            self.push_chunk(&chunk);
        }
        self.flush_pending_chunks();

        let root = build_balanced_root(self.leaves, self.version);
        #[cfg(any(feature = "profile-ipfs-cid", test))]
        let started = Instant::now();
        let out = match (self.version, root.codec) {
            (CidVersion::V0, MULTICODEC_DAG_PB) => multihash_to_base58btc(&root.multihash),
            _ => cid_to_base32(&root.cid),
        };
        #[cfg(any(feature = "profile-ipfs-cid", test))]
        record_profile(ProfilePhase::CidTextEncoding, started);
        DigestValue::from_raw_standard(root.cid, out)
    }
}

fn raw_block(data: &[u8]) -> UnixFsBlock {
    #[cfg(any(feature = "profile-ipfs-cid", test))]
    let started = Instant::now();
    let (cid, multihash) = cid_v1_bytes(MULTICODEC_RAW, data);
    #[cfg(any(feature = "profile-ipfs-cid", test))]
    record_profile(ProfilePhase::RawLeafHashing, started);
    UnixFsBlock {
        cid,
        multihash,
        codec: MULTICODEC_RAW,
        file_size: data.len() as u64,
        tsize: data.len() as u64,
    }
}

fn build_balanced_root(mut level: Vec<UnixFsBlock>, version: CidVersion) -> UnixFsBlock {
    while level.len() > 1 {
        level = level
            .par_chunks(MAX_LINKS)
            .map(|children| unixfs_file_block(children, version))
            .collect::<Vec<_>>();
    }
    level.pop().expect("root block must exist")
}

fn unixfs_file_block(children: &[UnixFsBlock], version: CidVersion) -> UnixFsBlock {
    #[cfg(any(feature = "profile-ipfs-cid", test))]
    let started = Instant::now();
    let file_size = children.iter().map(|child| child.file_size).sum();
    let data = unixfs_file_data(file_size, children.iter().map(|child| child.file_size));
    let links = children
        .iter()
        .map(|child| dag_pb_link(&child.cid, child.tsize))
        .collect::<Vec<_>>();
    let block = dag_pb_node(&data, &links);
    let child_tsize = children.iter().map(|child| child.tsize).sum::<u64>();
    #[cfg(any(feature = "profile-ipfs-cid", test))]
    record_profile(ProfilePhase::DagPbEncoding, started);
    let (cid, multihash) = dag_pb_cid_bytes(&block, version);
    UnixFsBlock {
        cid,
        multihash,
        codec: MULTICODEC_DAG_PB,
        file_size,
        tsize: block.len() as u64 + child_tsize,
    }
}

fn cid_v1_bytes(codec: u64, block: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let digest = Sha256::digest(block);
    let mut multihash = Vec::with_capacity(2 + digest.len());
    encode_varint(MULTIHASH_SHA2_256, &mut multihash);
    encode_varint(digest.len() as u64, &mut multihash);
    multihash.extend_from_slice(&digest);

    let mut out = Vec::with_capacity(2 + multihash.len());
    encode_varint(CID_VERSION, &mut out);
    encode_varint(codec, &mut out);
    out.extend_from_slice(&multihash);
    (out, multihash)
}

fn dag_pb_cid_bytes(block: &[u8], version: CidVersion) -> (Vec<u8>, Vec<u8>) {
    #[cfg(any(feature = "profile-ipfs-cid", test))]
    let started = Instant::now();
    let (cid, multihash) = cid_v1_bytes(MULTICODEC_DAG_PB, block);
    #[cfg(any(feature = "profile-ipfs-cid", test))]
    record_profile(ProfilePhase::DagPbHashing, started);
    match version {
        CidVersion::V0 => (multihash.clone(), multihash),
        CidVersion::V1 => (cid, multihash),
    }
}

fn cid_to_base32(cid: &[u8]) -> String {
    base32_lower_no_padding_multibase(cid)
}

fn multihash_to_base58btc(multihash: &[u8]) -> String {
    let mut digits = vec![0_u8];
    for &byte in multihash {
        let mut carry = byte as u32;
        for digit in &mut digits {
            let value = (*digit as u32 * 256) + carry;
            *digit = (value % 58) as u8;
            carry = value / 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    let zeroes = multihash.iter().take_while(|&&byte| byte == 0).count();
    let mut out = String::with_capacity(zeroes + digits.len());
    for _ in 0..zeroes {
        out.push('1');
    }
    for digit in digits.iter().rev() {
        out.push(BASE58BTC_ALPHABET[*digit as usize] as char);
    }
    out
}

fn unixfs_file_data(block_size: u64, blocksizes: impl IntoIterator<Item = u64>) -> Vec<u8> {
    let mut out = Vec::new();
    encode_key(1, 0, &mut out);
    encode_varint(UNIXFS_FILE_TYPE, &mut out);
    encode_key(3, 0, &mut out);
    encode_varint(block_size, &mut out);
    for size in blocksizes {
        encode_key(4, 0, &mut out);
        encode_varint(size, &mut out);
    }
    out
}

fn dag_pb_link(cid: &[u8], total_size: u64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_bytes_field(1, cid, &mut out);
    encode_bytes_field(2, b"", &mut out);
    encode_key(3, 0, &mut out);
    encode_varint(total_size, &mut out);
    out
}

fn dag_pb_node(data: &[u8], links: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for link in links {
        encode_bytes_field(2, link, &mut out);
    }
    encode_bytes_field(1, data, &mut out);
    out
}

fn encode_bytes_field(field: u64, bytes: &[u8], out: &mut Vec<u8>) {
    encode_key(field, 2, out);
    encode_varint(bytes.len() as u64, out);
    out.extend_from_slice(bytes);
}

fn encode_key(field: u64, wire_type: u64, out: &mut Vec<u8>) {
    encode_varint((field << 3) | wire_type, out);
}

fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cidv0(data: &[u8]) -> String {
        let mut h = CidHasher::v0();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    fn cidv1(data: &[u8]) -> String {
        let mut h = CidHasher::v1();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn empty_input_matches_kubo_nocopy_cidv0_raw_leaf() {
        assert_eq!(
            cidv0(b""),
            "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku"
        );
        assert_eq!(
            cidv1(b""),
            "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku"
        );
    }

    #[test]
    fn abc_matches_kubo_nocopy_cidv0_raw_leaf() {
        assert_eq!(
            cidv0(b"abc"),
            "bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu"
        );
        assert_eq!(
            cidv1(b"abc"),
            "bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu"
        );
    }

    #[test]
    fn cidv0_exactly_one_full_chunk_matches_kubo_nocopy_raw_leaf() {
        let data = vec![0; CHUNK_SIZE];
        assert_eq!(
            cidv0(&data),
            "bafkreiekhhjkxu4ztk3tyng3er3ijhg56mb44oe3gwbgquhzu4afrg2ksa"
        );
    }

    #[test]
    fn cidv1_exactly_one_full_chunk_matches_kubo_nocopy_raw_leaf() {
        let data = vec![0; CHUNK_SIZE];
        assert_eq!(
            cidv1(&data),
            "bafkreiekhhjkxu4ztk3tyng3er3ijhg56mb44oe3gwbgquhzu4afrg2ksa"
        );
    }

    #[test]
    fn cidv0_multi_chunk_matches_kubo_nocopy_default() {
        let data = vec![0; CHUNK_SIZE + 1];
        assert_eq!(
            cidv0(&data),
            "Qmc2SWxBGrBtWKZxuyg8999QuzXsPR47zsWiM7Yq9YFUXT"
        );
    }

    #[test]
    fn cidv1_multi_chunk_matches_kubo_nocopy_cid_version_1() {
        let data = vec![0; CHUNK_SIZE + 1];
        assert_eq!(
            cidv1(&data),
            "bafybeigllfqgfpqydppr6cmv56g7ax4wyhruzswvcefv6j5kj77nzttfki"
        );
    }

    #[test]
    fn cidv0_file_exceeding_single_node_fanout_matches_kubo_nocopy_default() {
        let data = vec![0; CHUNK_SIZE * (MAX_LINKS + 1)];
        assert_eq!(
            cidv0(&data),
            "QmVPDv3cu8fmHYUag4hCnNrUk2X9vMafgtZ6uFAjX1f2V2"
        );
    }

    #[test]
    fn base58btc_preserves_leading_zero_bytes() {
        assert_eq!(multihash_to_base58btc(&[0, 0, 1]), "112");
    }

    #[test]
    fn chunked_updates_match_single_update() {
        let data = vec![42; CHUNK_SIZE + 17];

        let mut one = CidHasher::v1();
        one.update(&data);

        let mut chunked = CidHasher::v1();
        for chunk in data.chunks(1021) {
            chunked.update(chunk);
        }

        assert_eq!(
            Box::new(one).finalize_hex(),
            Box::new(chunked).finalize_hex()
        );
    }

    #[test]
    fn parallel_batches_preserve_chunk_order_across_update_boundaries() {
        let data = (0..(CHUNK_SIZE * (PARALLEL_CHUNK_BATCH_SIZE + 3) + 17))
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();

        for new_hasher in [CidHasher::v0, CidHasher::v1] {
            let mut one = new_hasher();
            one.update(&data);

            let mut chunked = new_hasher();
            for chunk in data.chunks(8191) {
                chunked.update(chunk);
            }

            assert_eq!(
                Box::new(one).finalize_hex(),
                Box::new(chunked).finalize_hex()
            );
        }
    }

    #[test]
    fn profile_records_cid_hotspot_phases() {
        reset_profile();
        let data = vec![42; CHUNK_SIZE + 17];

        let mut hasher = CidHasher::v0();
        hasher.update(&data);
        let _ = Box::new(hasher).finalize_hex();

        let profile = take_profile();
        assert!(profile.chunk_buffering_ns > 0);
        assert!(profile.raw_leaf_hashing_ns > 0);
        assert!(profile.dag_pb_encoding_ns > 0);
        assert!(profile.dag_pb_hashing_ns > 0);
        assert!(profile.cid_text_encoding_ns > 0);
    }
}
