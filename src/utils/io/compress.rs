use std::env;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

use flate2::write::GzEncoder;
use flate2::Compression;
use xz2::write::XzEncoder;

#[derive(Debug)]
pub struct CompressResult {
    pub output_file: PathBuf,
    pub compressed_size: u64,
    pub elapsed_ms: u128,
}

pub fn compress_file(name: &str, codec: &str) -> io::Result<CompressResult> {
    let base = env::var("FILE_STORAGE_PATH").unwrap_or_else(|_| "./data/files".to_string());
    let input_path = PathBuf::from(&base).join(name);

    if !input_path.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Input file not found"));
    }

    let out_path = match codec {
        "gzip" => input_path.with_extension(format!("{}gz", input_path.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default())),
        "xz" => input_path.with_extension(format!("{}xz", input_path.extension()
            .map(|e| format!("{}.", e.to_string_lossy()))
            .unwrap_or_default())),
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported codec")),
    };

    let start = Instant::now();
    let infile = File::open(&input_path)?;
    let mut reader = BufReader::new(infile);
    let outfile = File::create(&out_path)?;
    let mut writer = BufWriter::new(outfile);

    match codec {
        "gzip" => {
            let mut encoder = GzEncoder::new(writer, Compression::default());
            io::copy(&mut reader, &mut encoder)?;
            writer = encoder.finish()?;
        }
        "xz" => {
            let mut encoder = XzEncoder::new(writer, 6); // level 6 compression
            io::copy(&mut reader, &mut encoder)?;
            writer = encoder.finish()?;
        }
        _ => unreachable!(),
    }

    writer.flush()?;
    let elapsed = start.elapsed().as_millis();

    let metadata = fs::metadata(&out_path)?;
    Ok(CompressResult {
        output_file: out_path,
        compressed_size: metadata.len(),
        elapsed_ms: elapsed,
    })
}

pub fn compress_json(name: &str, codec: &str) -> String {
    match compress_file(name, codec) {
        Ok(res) => format!(
            r#"{{"output":"{}","size_bytes":{},"elapsed_ms":{}}}"#,
            res.output_file.display(),
            res.compressed_size,
            res.elapsed_ms
        ),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn make_test_file(name: &str, size_kb: usize) -> PathBuf {
        let base = "./data/files";
        std::fs::create_dir_all(base).unwrap();
        let path = PathBuf::from(base).join(name);
        let mut f = File::create(&path).unwrap();
        for i in 0..size_kb {
            writeln!(f, "This is test line number {}", i).unwrap();
        }
        path
    }

    #[test]
    fn compress_gzip_basic() {
        let path = make_test_file("test_compress_gzip.txt", 5);
        let res = compress_file("test_compress_gzip.txt", "gzip").unwrap();
        assert!(res.output_file.exists());
        assert!(res.compressed_size > 0);
        fs::remove_file(res.output_file).ok();
        fs::remove_file(path).ok();
    }

    #[test]
    fn compress_xz_basic() {
        let path = make_test_file("test_compress_xz.txt", 5);
        let res = compress_file("test_compress_xz.txt", "xz").unwrap();
        assert!(res.output_file.exists());
        assert!(res.compressed_size > 0);
        fs::remove_file(res.output_file).ok();
        fs::remove_file(path).ok();
    }

    #[test]
    fn compress_invalid_codec() {
        let path = make_test_file("test_compress_invalid.txt", 1);
        let res = compress_file("test_compress_invalid.txt", "zip");
        assert!(res.is_err());
        fs::remove_file(path).ok();
    }

    #[test]
    fn compress_large_file() {
        let path = make_test_file("test_compress_large.txt", 200);
        let res = compress_file("test_compress_large.txt", "gzip").unwrap();
        assert!(res.compressed_size < fs::metadata(&path).unwrap().len());
        assert!(res.elapsed_ms < 3000, "Compression too slow");
        fs::remove_file(res.output_file).ok();
        fs::remove_file(path).ok();
    }
}
