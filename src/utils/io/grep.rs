use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::time::Instant;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct GrepResult {
    pub total_matches: usize,
    pub matched_lines: Vec<String>,
    pub elapsed_ms: u128,
}

pub fn grep_file(file_name: &str, pattern: &str) -> io::Result<GrepResult> {
    let base = env::var("FILE_STORAGE_PATH").unwrap_or_else(|_| "./data/files".to_string());
    let path = PathBuf::from(base).join(file_name);

    let file = File::open(&path)?;
    let reader = BufReader::with_capacity(128 * 1024, file); // 128KB buffered read

    let regex = Regex::new(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let mut total = 0usize;
    let mut matches = Vec::new();

    let start = Instant::now();

    for line_res in reader.lines() {
        let line = line_res?;
        if regex.is_match(&line) {
            total += 1;
            if matches.len() < 10 {
                matches.push(line);
            }
        }
    }

    Ok(GrepResult {
        total_matches: total,
        matched_lines: matches,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

/// Returns JSON-formatted string suitable for HTTP responses.
pub fn grep_json(file_name: &str, pattern: &str) -> String {
    match grep_file(file_name, pattern) {
        Ok(res) => {
            let lines_json = res.matched_lines
                .iter()
                .map(|l| format!("\"{}\"", l.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"{{"matches":{},"lines":[{}],"elapsed_ms":{}}}"#,
                res.total_matches, lines_json, res.elapsed_ms
            )
        }
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn make_file(name: &str, content: &str) -> PathBuf {
        let base = "./data/files";
        std::fs::create_dir_all(base).unwrap();
        let path = PathBuf::from(base).join(name);
        let mut f = File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn grep_no_matches() {
        let path = make_file("grep_none.txt", "apple\nbanana\ncarrot");
        let res = grep_file("grep_none.txt", "xyz").unwrap();
        assert_eq!(res.total_matches, 0);
        assert!(res.matched_lines.is_empty());
        fs::remove_file(path).ok();
    }

    #[test]
    fn grep_basic_matches() {
        let path = make_file("grep_basic.txt", "rust\nrocks\nrustacean\nRUST!");
        let res = grep_file("grep_basic.txt", "(?i)rust").unwrap(); // case-insensitive
        assert_eq!(res.total_matches, 3);
        assert_eq!(res.matched_lines.len(), 3);
        fs::remove_file(path).ok();
    }

    #[test]
    fn grep_limit_to_first_10() {
        let mut content = String::new();
        for i in 0..25 {
            content.push_str(&format!("match_line_{}\n", i));
        }
        let path = make_file("grep_limit.txt", &content);
        let res = grep_file("grep_limit.txt", "match_line_").unwrap();
        assert_eq!(res.total_matches, 25);
        assert_eq!(res.matched_lines.len(), 10);
        assert!(res.matched_lines[0].starts_with("match_line_0"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn grep_performance_large() {
        // 50_000 lines, ~1.5 MB
        let mut content = String::new();
        for i in 0..50_000 {
            if i % 10 == 0 {
                content.push_str("rust rocks!\n");
            } else {
                content.push_str("nothing here\n");
            }
        }
        let path = make_file("grep_large.txt", &content);
        let res = grep_file("grep_large.txt", "rust").unwrap();
        assert_eq!(res.total_matches, 5000);
        assert_eq!(res.matched_lines.len(), 10);
        assert!(res.elapsed_ms < 1000, "Should run under 1s");
        fs::remove_file(path).ok();
    }
}