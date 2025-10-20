
use rand::Rng;

pub fn fibonacci(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut a = 0;
    let mut b = 1;
    for _ in 1..n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
}

pub fn random(count: usize, min: i32, max: i32) -> Vec<i32> {
    let mut rng = rand::thread_rng();
    (0..count).map(|_| rng.gen_range(min..=max)).collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci() {
        assert_eq!(fibonacci(10), 55);
    }

    #[test]
    fn test_random_range() {
        let nums = random(5, 1, 10);
        assert_eq!(nums.len(), 5);
        assert!(nums.iter().all(|&n| n >= 1 && n <= 10));
    }
}
