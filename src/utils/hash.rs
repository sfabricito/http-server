
use sha2::{Sha256, Digest};

pub fn hash_text(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let h1 = hash_text("abc");
        let h2 = hash_text("abc");
        assert_eq!(h1, h2);
    }
}
