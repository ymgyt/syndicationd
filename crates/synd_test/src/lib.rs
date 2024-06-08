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

pub fn certificate_buff() -> String {
    std::fs::read_to_string(certificate()).unwrap()
}

pub fn private_key() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".dev")
        .join("self_signed_certs")
        .join("private_key.pem")
}

pub fn private_key_buff() -> Vec<u8> {
    std::fs::read(private_key()).unwrap()
}
