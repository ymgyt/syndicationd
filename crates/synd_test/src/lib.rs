use std::path::PathBuf;

pub mod kvsd;
pub mod mock;

pub fn certificate() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".dev")
        .join("self_signed_certs")
        .join("certificate.pem")
}

pub fn private_key() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".dev")
        .join("self_signed_certs")
        .join("private_key.pem")
}
