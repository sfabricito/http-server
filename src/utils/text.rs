
pub fn reverse(text: &str) -> String {
    text.chars().rev().collect()
}

pub fn to_upper(text: &str) -> String {
    text.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse() {
        assert_eq!(reverse("abcd"), "dcba");
    }

    #[test]
    fn test_to_upper() {
        assert_eq!(to_upper("rust"), "RUST");
    }
}
