use clap::Parser;
use hashjunkie_core::{Algorithm, UnknownAlgorithm};

#[derive(Parser, Debug)]
#[command(name = "hashjunkie", version, about = "Multi-algorithm file hasher")]
pub struct Args {
    /// Files to hash (omit to read from stdin)
    pub files: Vec<String>,

    /// Comma-separated list of algorithms (default: all except opt-in whirlpool)
    #[arg(short = 'a', long = "algorithms")]
    pub algorithms: Option<String>,

    /// Output format
    #[arg(short = 'f', long = "format", default_value = "json")]
    pub format: Format,
}

impl Args {
    pub fn resolved_algorithms(&self) -> Result<Vec<Algorithm>, UnknownAlgorithm> {
        match &self.algorithms {
            None => Ok(Algorithm::all().to_vec()),
            Some(s) => parse_algorithms(s),
        }
    }
}

fn parse_algorithms(s: &str) -> Result<Vec<Algorithm>, UnknownAlgorithm> {
    s.split(',')
        .map(|part| part.trim().parse::<Algorithm>())
        .collect()
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Format {
    Json,
    Hex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_known_algorithm() {
        let algs = parse_algorithms("sha256").unwrap();
        assert_eq!(algs, vec![Algorithm::Sha256]);
    }

    #[test]
    fn parse_multiple_algorithms() {
        let algs = parse_algorithms("sha256,md5").unwrap();
        assert_eq!(algs, vec![Algorithm::Sha256, Algorithm::Md5]);
    }

    #[test]
    fn parse_trims_whitespace() {
        let algs = parse_algorithms("sha256 , md5").unwrap();
        assert_eq!(algs, vec![Algorithm::Sha256, Algorithm::Md5]);
    }

    #[test]
    fn parse_unknown_algorithm_returns_error() {
        let err = parse_algorithms("bogus").unwrap_err();
        assert_eq!(err.to_string(), "unknown algorithm: bogus");
    }

    #[test]
    fn resolved_algorithms_none_returns_default_18_without_whirlpool() {
        let args = Args::parse_from(["hashjunkie"]);
        let algs = args.resolved_algorithms().unwrap();
        assert_eq!(algs.len(), 18);
        assert!(algs.contains(&Algorithm::Aich));
        assert!(algs.contains(&Algorithm::Ed2k));
        assert!(algs.contains(&Algorithm::Tiger));
        assert!(!algs.contains(&Algorithm::Whirlpool));
    }

    #[test]
    fn resolved_algorithms_with_list_returns_subset() {
        let args = Args::parse_from(["hashjunkie", "-a", "sha256,md5"]);
        let algs = args.resolved_algorithms().unwrap();
        assert_eq!(algs, vec![Algorithm::Sha256, Algorithm::Md5]);
    }
}
