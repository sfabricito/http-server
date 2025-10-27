
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

#[inline]
pub fn mul_mod_u128(a: u128, b: u128, m: u128) -> u128 {
    ((a % m) * (b % m)) % m
}

#[inline]
pub fn pow_mod_u128(mut a: u128, mut e: u128, m: u128) -> u128 {
    let mut r: u128 = 1 % m;
    a %= m;
    while e > 0 {
        if (e & 1) == 1 { r = mul_mod_u128(r, a, m); }
        a = mul_mod_u128(a, a, m);
        e >>= 1;
    }
    r
}

#[inline]
pub fn mul_mod_u64(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

#[inline]
pub fn pow_mod_u64(mut a: u64, mut e: u64, m: u64) -> u64 {
    let mut r: u64 = 1 % m;
    a %= m;
    while e > 0 {
        if (e & 1) == 1 { r = mul_mod_u64(r, a, m); }
        a = mul_mod_u64(a, a, m);
        e >>= 1;
    }
    r
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
