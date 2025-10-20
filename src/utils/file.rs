
use std::fs::{File, remove_file};
use std::io::{Write, Result};

pub fn create_file(name: &str, content: &str, repeat: usize) -> Result<()> {
    let mut file = File::create(name)?;
    for _ in 0..repeat {
        writeln!(file, "{}", content)?;
    }
    Ok(())
}

pub fn delete_file(name: &str) -> Result<()> {
    remove_file(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_and_delete_file() {
        let name = "test_output.txt";
        create_file(name, "Hello", 2).unwrap();
        let content = fs::read_to_string(name).unwrap();
        assert!(content.contains("Hello"));
        delete_file(name).unwrap();
        assert!(!fs::metadata(name).is_ok());
    }
}
