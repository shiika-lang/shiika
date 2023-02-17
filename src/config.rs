use std::env;
use std::path::PathBuf;

pub fn from_shiika_root(s: &str) -> PathBuf {
    shiika_root().join(s)
}

fn shiika_root() -> PathBuf {
    PathBuf::from(env::var("SHIIKA_ROOT").unwrap_or_else(|_| ".".to_string()))
}
