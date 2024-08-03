use std::path::PathBuf;

pub mod jwt;
pub mod kvsd;
pub mod mock;

pub const TEST_EMAIL: &str = "ymgyt@ymgyt.io";
pub const TEST_USER_ID: &str = "899cf3fa5afc0aa1";
pub const GITHUB_INVALID_TOKEN: &str = "github_invalid_token";

pub fn certificate() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("etc")
        .join("dev")
        .join("self_signed_certs")
        .join("certificate.pem")
}

pub fn certificate_buff() -> String {
    std::fs::read_to_string(certificate()).unwrap()
}

pub fn private_key() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("etc")
        .join("dev")
        .join("self_signed_certs")
        .join("private_key.pem")
}

pub fn private_key_buff() -> Vec<u8> {
    std::fs::read(private_key()).unwrap()
}

pub fn temp_dir() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}
