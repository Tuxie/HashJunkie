use crate::hashes::Hasher;

const WIDTH_BITS: u64 = 160;
const SHIFT: u64 = 11;

pub struct QuickXorHasher {
    state: [u8; 20],
    length: u64,
    bit_offset: u64,
}

impl QuickXorHasher {
    pub fn new() -> Self {
        Self {
            state: [0u8; 20],
            length: 0,
            bit_offset: 0,
        }
    }
}

impl Default for QuickXorHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for QuickXorHasher {
    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let start_bit = self.bit_offset % WIDTH_BITS;
            let byte_index = (start_bit / 8) as usize;
            let bit_in_byte = (start_bit % 8) as u8;

            self.state[byte_index % 20] ^= byte << bit_in_byte;
            if bit_in_byte > 0 {
                self.state[(byte_index + 1) % 20] ^= byte >> (8 - bit_in_byte);
            }

            self.bit_offset += SHIFT;
        }
        self.length += data.len() as u64;
    }

    fn finalize_hex(mut self: Box<Self>) -> String {
        // XOR 8-byte little-endian length into state bytes 0–7
        for (i, b) in self.length.to_le_bytes().iter().enumerate() {
            self.state[i] ^= b;
        }
        hex::encode(self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashes::Hasher;

    fn hash(data: &[u8]) -> String {
        let mut h = QuickXorHasher::new();
        h.update(data);
        Box::new(h).finalize_hex()
    }

    #[test]
    fn quickxor_empty() {
        // Empty: state stays all-zeros, length=0 XORed in → still all-zeros
        assert_eq!(hash(b""), "0000000000000000000000000000000000000000");
    }

    #[test]
    fn quickxor_output_is_40_hex_chars() {
        assert_eq!(hash(b"test").len(), 40);
    }

    #[test]
    fn chunked_matches_single() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let single = hash(data);
        let mut h = QuickXorHasher::new();
        for chunk in data.chunks(7) {
            h.update(chunk);
        }
        assert_eq!(Box::new(h).finalize_hex(), single);
    }

    #[test]
    fn single_byte_a() {
        // Byte 'a' (0x61) at bit_pos 0: byte_idx=0, bit_offset=0
        // state[0] ^= 0x61 << 0 = 0x61, no second byte (bit_in_byte == 0)
        // Length = 1: state[0] ^= 1 → 0x61 ^ 0x01 = 0x60
        // All other bytes zero
        let result = hash(b"a");
        assert_eq!(&result[0..2], "60", "first byte should be 0x60");
        assert_eq!(
            &result[2..],
            "00".repeat(19),
            "remaining bytes should be zero"
        );
    }
}
