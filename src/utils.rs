use sha2::{Digest, Sha512};

pub fn hash_str(path: &str) -> Result<String, std::io::Error> {
    let mut hasher = Sha512::new();
    hasher.update(path.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}
