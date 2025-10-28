use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::Instant;

pub fn sort_file(name: &str, algo: &str) -> io::Result<(PathBuf, usize, u128)> {
    let base = env::var("FILE_STORAGE_PATH").unwrap_or_else(|_| "./data/files".to_string());
    let path = PathBuf::from(base).join(name);

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut numbers: Vec<i64> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if let Ok(n) = line.trim().parse::<i64>() {
            numbers.push(n);
        }
    }

    let start = Instant::now();

    match algo {
        "merge" => merge_sort(&mut numbers),
        "quick" => quick_sort(&mut numbers),
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "Unknown algorithm")),
    }

    let elapsed = start.elapsed().as_millis();

    let out_path = {
        let mut p = path.clone();
        p.set_file_name(format!("{}_sorted_{}", name, algo));
        p
    };

    let out_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&out_path)?;
    let mut writer = BufWriter::new(out_file);
    for n in &numbers {
        writeln!(writer, "{}", n)?;
    }
    writer.flush()?;

    Ok((out_path, numbers.len(), elapsed))

}

fn merge_sort(arr: &mut [i64]) {
    let len = arr.len();
    if len <= 1 {
        return;
    }
    let mid = len / 2;
    let mut left = arr[..mid].to_vec();
    let mut right = arr[mid..].to_vec();
    merge_sort(&mut left);
    merge_sort(&mut right);

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            arr[k] = left[i];
            i += 1;
        } else {
            arr[k] = right[j];
            j += 1;
        }
        k += 1;
    }
    while i < left.len() {
        arr[k] = left[i];
        i += 1;
        k += 1;
    }
    while j < right.len() {
        arr[k] = right[j];
        j += 1;
        k += 1;
    }
}

fn quick_sort(arr: &mut [i64]) {
    if arr.len() <= 1 {
        return;
    }
    let pivot_index = partition(arr);
    let (left, right) = arr.split_at_mut(pivot_index);
    quick_sort(left);
    quick_sort(&mut right[1..]);
}

fn partition(arr: &mut [i64]) -> usize {
    let len = arr.len();
    let pivot = arr[len - 1];
    let mut i = 0;
    for j in 0..len - 1 {
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, len - 1);
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn setup_temp_file(values: &[i64], name: &str) -> PathBuf {
        let base = "./data/files";
        std::fs::create_dir_all(base).unwrap();
        let path = PathBuf::from(base).join(name);
        let mut f = File::create(&path).unwrap();
        for v in values {
            writeln!(f, "{}", v).unwrap();
        }
        path
    }

    #[test]
    fn test_merge_sort_file() {
        let path = setup_temp_file(&[5, 1, 3, 2], "merge_test.txt");
        let (out, count, _) = sort_file("merge_test.txt", "merge").unwrap();
        assert_eq!(count, 4);
        let content = fs::read_to_string(out).unwrap();
        assert_eq!(content.trim(), "1\n2\n3\n5");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_quick_sort_file() {
        let path = setup_temp_file(&[10, -1, 4, 7], "quick_test.txt");
        let (out, count, _) = sort_file("quick_test.txt", "quick").unwrap();
        assert_eq!(count, 4);
        let content = fs::read_to_string(out).unwrap();
        assert_eq!(content.trim(), "-1\n4\n7\n10");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_invalid_algorithm() {
        let path = setup_temp_file(&[1, 2, 3], "invalid_test.txt");
        let err = sort_file("invalid_test.txt", "bogus").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        fs::remove_file(path).unwrap();
    }
}
