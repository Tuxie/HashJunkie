use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    Aich,
    Blake3,
    Btv2,
    CidV0,
    CidV1,
    Crc32,
    Dropbox,
    Ed2k,
    Hidrive,
    Mailru,
    Md5,
    QuickXor,
    Sha1,
    Sha256,
    Sha512,
    Tiger,
    Whirlpool,
    Xxh128,
    Xxh3,
}

impl Algorithm {
    pub fn supported() -> &'static [Algorithm] {
        &[
            Algorithm::Aich,
            Algorithm::Blake3,
            Algorithm::Btv2,
            Algorithm::CidV0,
            Algorithm::CidV1,
            Algorithm::Crc32,
            Algorithm::Dropbox,
            Algorithm::Ed2k,
            Algorithm::Hidrive,
            Algorithm::Mailru,
            Algorithm::Md5,
            Algorithm::QuickXor,
            Algorithm::Sha1,
            Algorithm::Sha256,
            Algorithm::Sha512,
            Algorithm::Tiger,
            Algorithm::Whirlpool,
            Algorithm::Xxh128,
            Algorithm::Xxh3,
        ]
    }

    pub fn all() -> &'static [Algorithm] {
        &[
            Algorithm::Aich,
            Algorithm::Blake3,
            Algorithm::Btv2,
            Algorithm::CidV0,
            Algorithm::CidV1,
            Algorithm::Crc32,
            Algorithm::Dropbox,
            Algorithm::Ed2k,
            Algorithm::Hidrive,
            Algorithm::Mailru,
            Algorithm::Md5,
            Algorithm::QuickXor,
            Algorithm::Sha1,
            Algorithm::Sha256,
            Algorithm::Sha512,
            Algorithm::Tiger,
            Algorithm::Xxh128,
            Algorithm::Xxh3,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Aich => "aich",
            Algorithm::Blake3 => "blake3",
            Algorithm::Btv2 => "btv2",
            Algorithm::CidV0 => "cidv0",
            Algorithm::CidV1 => "cidv1",
            Algorithm::Crc32 => "crc32",
            Algorithm::Dropbox => "dropbox",
            Algorithm::Ed2k => "ed2k",
            Algorithm::Hidrive => "hidrive",
            Algorithm::Mailru => "mailru",
            Algorithm::Md5 => "md5",
            Algorithm::QuickXor => "quickxor",
            Algorithm::Sha1 => "sha1",
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha512 => "sha512",
            Algorithm::Tiger => "tiger",
            Algorithm::Whirlpool => "whirlpool",
            Algorithm::Xxh128 => "xxh128",
            Algorithm::Xxh3 => "xxh3",
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug)]
pub struct UnknownAlgorithm(pub String);

impl fmt::Display for UnknownAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown algorithm: {}", self.0)
    }
}

impl std::error::Error for UnknownAlgorithm {}

impl std::str::FromStr for Algorithm {
    type Err = UnknownAlgorithm;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aich" => Ok(Algorithm::Aich),
            "blake3" => Ok(Algorithm::Blake3),
            "btv2" => Ok(Algorithm::Btv2),
            "cidv0" => Ok(Algorithm::CidV0),
            "cidv1" => Ok(Algorithm::CidV1),
            "crc32" => Ok(Algorithm::Crc32),
            "dropbox" => Ok(Algorithm::Dropbox),
            "ed2k" => Ok(Algorithm::Ed2k),
            "hidrive" => Ok(Algorithm::Hidrive),
            "mailru" => Ok(Algorithm::Mailru),
            "md5" => Ok(Algorithm::Md5),
            "quickxor" => Ok(Algorithm::QuickXor),
            "sha1" => Ok(Algorithm::Sha1),
            "sha256" => Ok(Algorithm::Sha256),
            "sha512" => Ok(Algorithm::Sha512),
            "tiger" => Ok(Algorithm::Tiger),
            "whirlpool" => Ok(Algorithm::Whirlpool),
            "xxh128" => Ok(Algorithm::Xxh128),
            "xxh3" => Ok(Algorithm::Xxh3),
            other => Err(UnknownAlgorithm(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn all_returns_default_algorithms_without_whirlpool() {
        assert_eq!(Algorithm::all().len(), 18);
        assert!(!Algorithm::all().contains(&Algorithm::Whirlpool));
    }

    #[test]
    fn supported_returns_all_19_algorithms_including_whirlpool() {
        assert_eq!(Algorithm::supported().len(), 19);
        assert!(Algorithm::supported().contains(&Algorithm::Ed2k));
        assert!(Algorithm::supported().contains(&Algorithm::Tiger));
        assert!(Algorithm::supported().contains(&Algorithm::Whirlpool));
    }

    #[test]
    fn display_roundtrips_via_from_str() {
        for alg in Algorithm::supported() {
            let s = alg.to_string();
            let parsed = Algorithm::from_str(&s).unwrap();
            assert_eq!(*alg, parsed);
        }
    }

    #[test]
    fn unknown_algorithm_returns_error() {
        assert!(Algorithm::from_str("bogus").is_err());
    }

    #[test]
    fn as_str_matches_display() {
        for alg in Algorithm::supported() {
            assert_eq!(alg.as_str(), alg.to_string());
        }
    }

    #[test]
    fn unknown_algorithm_display_message() {
        let err = UnknownAlgorithm("nope".to_string());
        assert_eq!(err.to_string(), "unknown algorithm: nope");
    }
}
