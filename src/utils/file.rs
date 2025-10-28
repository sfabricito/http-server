use std::env;
use std::fs::{self, File, remove_file, metadata};
use std::io::{Write, Result, ErrorKind};
use std::path::{Path, PathBuf};

/// Resolve a safe, absolute path using the FILE_STORAGE_PATH environment variable.
/// Falls back to "./data/" if the variable is missing.
fn resolve_path(filename: &str) -> PathBuf {
    let base = env::var("FILE_STORAGE_PATH").unwrap_or_else(|_| "./data/".to_string());
    let mut path = PathBuf::from(base);

    // Prevent directory traversal (e.g., "../../etc/passwd")
    let clean_name = Path::new(filename)
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("invalid"))
        .to_os_string();

    path.push(clean_name);
    path
}

pub fn create_file(name: &str, content: &str, repeat: usize) -> Result<()> {
    let path = resolve_path(name);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?; // ensure directory exists
    }

    let mut file = File::create(&path)?;
    for _ in 0..repeat {
        writeln!(file, "{}", content)?;
    }

    Ok(())
}

pub fn delete_file(name: &str) -> Result<String> {
    let path = resolve_path(name);

    match metadata(&path) {
        Ok(_) => match remove_file(&path) {
            Ok(_) => Ok(format!("File '{}' deleted successfully", name)),
            Err(e) => Err(e),
        },
        Err(e) if e.kind() == ErrorKind::NotFound => {
            Ok(format!("File '{}' does not exist", name))
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_env() {
        env::set_var("FILE_STORAGE_PATH", "./test_data/");
        fs::create_dir_all("./test_data/").unwrap();
    }

    #[test]
    fn test_create_and_delete_file() {
        setup_env();

        let name = "test_output.txt";
        create_file(name, "Hello", 2).unwrap();

        let path = resolve_path(name);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Hello"));

        let res = delete_file(name).unwrap();
        assert!(res.contains("deleted successfully") || res.contains("does not exist"));
        assert!(!fs::metadata(&path).is_ok());
    }

    #[test]
    fn test_delete_nonexistent_file() {
        setup_env();
        let name = "nonexistent.txt";
        let result = delete_file(name).unwrap();
        assert!(result.contains("does not exist"));
    }
}
