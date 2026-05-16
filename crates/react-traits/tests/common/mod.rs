use std::path::PathBuf;

pub fn fixture(category: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(category)
        .join(name)
}
