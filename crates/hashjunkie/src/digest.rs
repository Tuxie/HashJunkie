#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestValue {
    raw: Vec<u8>,
    standard: String,
}

impl DigestValue {
    pub fn from_raw_standard(raw: impl Into<Vec<u8>>, standard: impl Into<String>) -> Self {
        Self {
            raw: raw.into(),
            standard: standard.into(),
        }
    }

    pub fn from_raw_hex(raw: impl Into<Vec<u8>>) -> Self {
        let raw = raw.into();
        let standard = bytes_to_lower_hex(&raw);
        Self { raw, standard }
    }

    pub fn from_hex(hex: impl AsRef<str>) -> Result<Self, hex::FromHexError> {
        let standard = hex.as_ref().to_ascii_lowercase();
        let raw = hex::decode(&standard)?;
        Ok(Self { raw, standard })
    }

    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    pub fn into_raw(self) -> Vec<u8> {
        self.raw
    }

    pub fn standard(&self) -> &str {
        &self.standard
    }

    pub fn hex(&self) -> String {
        bytes_to_lower_hex(&self.raw)
    }
}

pub fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

pub fn base32_upper_no_padding(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    base32_no_padding(bytes, ALPHABET)
}

pub fn base32_lower_no_padding_multibase(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut out = String::with_capacity(1 + (bytes.len() * 8).div_ceil(5));
    out.push('b');
    out.push_str(&base32_no_padding(bytes, ALPHABET));
    out
}

fn base32_no_padding(bytes: &[u8], alphabet: &[u8; 32]) -> String {
    let mut out = String::with_capacity((bytes.len() * 8).div_ceil(5));
    let mut buffer = 0u16;
    let mut bits = 0u8;

    for byte in bytes {
        buffer = (buffer << 8) | u16::from(*byte);
        bits += 8;

        while bits >= 5 {
            let shift = bits - 5;
            let index = ((buffer >> shift) & 0x1f) as usize;
            out.push(alphabet[index] as char);
            bits -= 5;
            buffer &= (1 << bits) - 1;
        }
    }

    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(alphabet[index] as char);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_digest_keeps_standard_and_raw_in_sync() {
        let digest = DigestValue::from_hex("BA7816").unwrap();
        assert_eq!(digest.standard(), "ba7816");
        assert_eq!(digest.raw(), &[0xba, 0x78, 0x16]);
        assert_eq!(digest.hex(), "ba7816");
        assert_eq!(digest.into_raw(), vec![0xba, 0x78, 0x16]);
    }

    #[test]
    fn base32_upper_matches_aich_vector() {
        let raw = hex::decode("a9993e364706816aba3e25717850c26c9cd0d89d").unwrap();
        assert_eq!(
            base32_upper_no_padding(&raw),
            "VGMT4NSHA2AWVOR6EVYXQUGCNSONBWE5"
        );
    }
}
