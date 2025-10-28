use std::env;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WcCounts {
    pub lines: u64,
    pub words: u64,
    pub bytes: u64,
}

pub fn word_count(name: &str) -> io::Result<(WcCounts, u128, PathBuf)> {
    let base = env::var("FILE_STORAGE_PATH").unwrap_or_else(|_| "./data/files".to_string());
    let path = PathBuf::from(base).join(name);

    let file = File::open(&path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file); // 64 KiB chunks

    let start = Instant::now();

    let mut buf = [0u8; 64 * 1024];
    let mut counts = WcCounts { lines: 0, words: 0, bytes: 0 };
    let mut in_word = false;

    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }

        counts.bytes += read as u64;

        for &b in &buf[..read] {
            if b == b'\n' {
                counts.lines += 1;
            }
            let is_ws = matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B /* VT */ | 0x0C /* FF */);
            if is_ws {
                in_word = false;
            } else if !in_word {
                counts.words += 1;
                in_word = true;
            }
        }
    }

    let elapsed = start.elapsed().as_millis();
    Ok((counts, elapsed, path))
}

pub fn wordcount_json(name: &str) -> String {
    match word_count(name) {
        Ok((c, elapsed, path)) => {
            format!(
                r#"{{"file":"{}","lines":{},"words":{},"bytes":{},"elapsed_ms":{}}}"#,
                path.display(), c.lines, c.words, c.bytes, elapsed
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

    fn write_file(rel_name: &str, data: &[u8]) -> PathBuf {
        let base = "./data/files";
        std::fs::create_dir_all(base).unwrap();
        let path = PathBuf::from(base).join(rel_name);
        let mut f = File::create(&path).unwrap();
        f.write_all(data).unwrap();
        path
    }

    #[test]
    fn wc_empty_file() {
        let p = write_file("wc_empty.txt", b"");
        let (c, _ms, _path) = word_count("wc_empty.txt").unwrap();
        assert_eq!(c.lines, 0);
        assert_eq!(c.words, 0);
        assert_eq!(c.bytes, 0);
        fs::remove_file(p).ok();
    }

    #[test]
    fn wc_simple_text() {
        // 2 lines (one newline), 5 words, 24 bytes
        // Words: "hello"(5) "world"(5) "this"(4) "is"(2) "rust!"(5)
        let data = b"hello world\nthis is rust!";
        let p = write_file("wc_simple.txt", data);
        let (c, _ms, _path) = word_count("wc_simple.txt").unwrap();
        assert_eq!(c.lines, 1);
        assert_eq!(c.words, 5);
        assert_eq!(c.bytes, data.len() as u64);
        fs::remove_file(p).ok();
    }

    #[test]
    fn wc_tricky_whitespace() {
        // Spaces, tabs, CR, LF, vertical tab, form feed
        // Text: "a\tb  \r\nc\x0B\x0Cd\n" => words: a,b,c,d; lines: 2 ('\n' twice)
        let data = b"a\tb  \r\nc\x0B\x0Cd\n";
        let p = write_file("wc_ws.txt", data);
        let (c, _ms, _path) = word_count("wc_ws.txt").unwrap();
        assert_eq!(c.lines, 2);
        assert_eq!(c.words, 4);
        assert_eq!(c.bytes, data.len() as u64);
        fs::remove_file(p).ok();
    }

    #[test]
    fn wc_no_trailing_newline() {
        // 0 or more lines depending on '\n' presence; last line without '\n' still counts words.
        // Here: one line, three words, no trailing '\n'
        let data = b"alpha beta gamma";
        let p = write_file("wc_no_nl.txt", data);
        let (c, _ms, _path) = word_count("wc_no_nl.txt").unwrap();
        assert_eq!(c.lines, 0); // no '\n'
        assert_eq!(c.words, 3);
        assert_eq!(c.bytes, data.len() as u64);
        fs::remove_file(p).ok();
    }

    #[test]
    fn wc_large_like() {
        // Build ~1MB of data to exercise chunking.
        let mut s = Vec::new();
        for _ in 0..50_000 {
            s.extend_from_slice(b"word1 word2 word3\n");
        }
        let p = write_file("wc_large.txt", &s);
        let (c, _ms, _path) = word_count("wc_large.txt").unwrap();
        assert_eq!(c.lines, 50_000);
        assert_eq!(c.words, 50_000 * 3);
        assert_eq!(c.bytes, s.len() as u64);
        fs::remove_file(p).ok();
    }
}
