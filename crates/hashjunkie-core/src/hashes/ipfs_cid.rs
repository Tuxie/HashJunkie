use digest::Digest;
use sha2::Sha256;

use super::Hasher;

const CHUNK_SIZE: usize = 262_144;
const MAX_LINKS: usize = 174;
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
    leaves: Vec<UnixFsBlock>,
    version: CidVersion,
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
            leaves: Vec::new(),
            version,
        }
    }

    fn push_chunk(&mut self, chunk: &[u8]) {
        self.leaves.push(raw_block(chunk));
    }
}

impl Hasher for CidHasher {
    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let remaining = CHUNK_SIZE - self.current.len();
            let take = remaining.min(data.len());
            self.current.extend_from_slice(&data[..take]);
            data = &data[take..];

            if self.current.len() == CHUNK_SIZE {
                let chunk = std::mem::take(&mut self.current);
                self.push_chunk(&chunk);
            }
        }
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        if !self.current.is_empty() || self.leaves.is_empty() {
            let chunk = std::mem::take(&mut self.current);
            self.push_chunk(&chunk);
        }

        let root = build_balanced_root(self.leaves);
        match (self.version, root.codec) {
            (CidVersion::V0, MULTICODEC_DAG_PB) => multihash_to_base58btc(&root.multihash),
            _ => cid_to_base32(&root.cid),
        }
    }
}

fn raw_block(data: &[u8]) -> UnixFsBlock {
    let (cid, multihash) = cid_bytes(MULTICODEC_RAW, data);
    UnixFsBlock {
        cid,
        multihash,
        codec: MULTICODEC_RAW,
        file_size: data.len() as u64,
        tsize: data.len() as u64,
    }
}

fn build_balanced_root(mut level: Vec<UnixFsBlock>) -> UnixFsBlock {
    while level.len() > 1 {
        level = level
            .chunks(MAX_LINKS)
            .map(unixfs_file_block)
            .collect::<Vec<_>>();
    }
    level.pop().expect("root block must exist")
}

fn unixfs_file_block(children: &[UnixFsBlock]) -> UnixFsBlock {
    let file_size = children.iter().map(|child| child.file_size).sum();
    let data = unixfs_file_data(file_size, children.iter().map(|child| child.file_size));
    let links = children
        .iter()
        .map(|child| dag_pb_link(&child.cid, child.tsize))
        .collect::<Vec<_>>();
    let block = dag_pb_node(&data, &links);
    let child_tsize = children.iter().map(|child| child.tsize).sum::<u64>();
    let (cid, multihash) = cid_bytes(MULTICODEC_DAG_PB, &block);
    UnixFsBlock {
        cid,
        multihash,
        codec: MULTICODEC_DAG_PB,
        file_size,
        tsize: block.len() as u64 + child_tsize,
    }
}

fn cid_bytes(codec: u64, block: &[u8]) -> (Vec<u8>, Vec<u8>) {
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

fn cid_to_base32(cid: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut out = String::with_capacity(1 + cid.len() * 8_usize.div_ceil(5));
    out.push('b');

    let mut value: u16 = 0;
    let mut bits = 0;
    for &byte in cid {
        value = (value << 8) | byte as u16;
        bits += 8;
        while bits >= 5 {
            let idx = ((value >> (bits - 5)) & 0x1f) as usize;
            out.push(ALPHABET[idx] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        let idx = ((value << (5 - bits)) & 0x1f) as usize;
        out.push(ALPHABET[idx] as char);
    }

    out
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
    fn empty_input_matches_raw_sha2_256_cid() {
        assert_eq!(
            cidv0(b""),
            "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku"
        );
        assert_eq!(cidv0(b""), cidv1(b""));
    }

    #[test]
    fn abc_matches_raw_sha2_256_cid() {
        assert_eq!(
            cidv1(b"abc"),
            "bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu"
        );
        assert_eq!(cidv0(b"abc"), cidv1(b"abc"));
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
}
