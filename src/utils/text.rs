
pub fn reverse(text: &str) -> String {
    text.chars().rev().collect()
}

pub fn to_upper(text: &str) -> String {
    text.to_ascii_uppercase()
}

pub fn help() -> String {
    let commands = [
        // --- CPU-bound endpoints ---
        "/isprime?n=NUM",
        "/factor?n=NUM",
        "/pi?digits=D",
        "/matrixmul?size=N&seed=S",
        "/mandelbrot?width=W&height=H&max_iter=I",

        // --- IO-bound endpoints ---
        "/createfile?name=filename&content=text&repeat=x",
        "/deletefile?name=filename",
        "/random?count=n&min=a&max=b",
        "/sleep?seconds=s",
        "/timestamp",

        // --- Command / Utility endpoints ---
        "/fibonacci?num=N",
        "/reverse?text=abcdef",
        "/toupper?text=abcd",
        "/hash?text=someinput",
        "/simulate?seconds=s&task=name",
        "/loadtest?tasks=n&sleep=x",
        "/status",
        "/help",

        // --- Job system endpoints ---
        "/jobs/submit?task=TASK&<params>",
        "/jobs/cancel?id=JOBID",
        "/jobs/result?id=JOBID",
        "/jobs/status?id=JOBID",
        "/metrics",
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
