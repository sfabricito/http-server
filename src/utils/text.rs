
pub fn reverse(text: &str) -> String {
    text.chars().rev().collect()
}

pub fn to_upper(text: &str) -> String {
    text.to_ascii_uppercase()
}

pub fn help() -> String {
    let commands = [
        "/fibonacci?num=N",
        "/createfile?name=filename&content=text&repeat=x",
        "/deletefile?name=filename",
        "/status",
        "/reverse?text=abcdef",
        "/toupper?text=abcd",
        "/random?count=n&min=a&max=b",
        "/timestamp",
        "/hash?text=someinput",
        "/simulate?seconds=s&task=name",
        "/sleep?seconds=s",
        "/loadtest?tasks=n&sleep=x",
        "/help",
    ];

    let mut output = String::from("Available commands:\n");
    for cmd in commands.iter() {
        output.push_str(&format!(" - {}\n", cmd));
    }

    output
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

    #[test]
    fn test_help() {
        let help_text = help();
        assert!(help_text.contains("/fibonacci?num=N"));
        assert!(help_text.contains("/help"));
        assert!(help_text.starts_with("Available commands:"));
    }
}
