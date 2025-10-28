use crate::utils::math::mul_mod_u64;
use crate::utils::cpu::is_prime::check;
use rand::Rng;
use std::collections::HashMap;

pub fn factorize(mut n: u64) -> Vec<(u64, u32)> {
    let mut factors = Vec::new();
    if n < 2 {
        return factors;
    }

    for &p in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47] {
        if n % p == 0 {
            let mut count = 0;
            while n % p == 0 {
                n /= p;
                count += 1;
            }
            factors.push((p, count));
        }
    }

    if n > 1 {
        factor_recursive(n, &mut factors);
    }

    let mut map = HashMap::new();
    for (p, c) in factors {
        *map.entry(p).or_insert(0) += c;
    }

    let mut merged: Vec<(u64, u32)> = map.into_iter().collect();
    merged.sort_by_key(|x| x.0);
    merged
}

fn factor_recursive(n: u64, factors: &mut Vec<(u64, u32)>) {
    if n == 1 {
        return;
    }
    if check(n) {
        factors.push((n, 1));
        return;
    }

    let d = pollards_rho(n);
    factor_recursive(d, factors);
    factor_recursive(n / d, factors);
}

fn pollards_rho(n: u64) ->   u64 {
    if n % 2 == 0 {
        return 2;
    }

    let mut rng = rand::thread_rng();
    let mut x: u64 = rng.gen_range(2..n - 1);
    let mut y = x;
    let c: u64 = rng.gen_range(1..n - 1);
    let mut d: u64 = 1;

    while d == 1 {
        x = (mul_mod_u64(x, x, n) + c) % n;
        y = (mul_mod_u64(y, y, n) + c) % n;
        y = (mul_mod_u64(y, y, n) + c) % n;
        let diff = if x > y { x - y } else { y - x };
        d = gcd_u64(diff, n);
        if d == n {
            return pollards_rho(n);
        }
    }
    d
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_factors() {
        assert_eq!(factorize(1), vec![]);
        assert_eq!(factorize(2), vec![(2, 1)]);
        assert_eq!(factorize(3), vec![(3, 1)]);
        assert_eq!(factorize(4), vec![(2, 2)]);
        assert_eq!(factorize(360), vec![(2, 3), (3, 2), (5, 1)]);
        assert_eq!(factorize(1000000), vec![(2, 6), (5, 6)]);
    }

    #[test]
    fn test_large_semiprimes() {
        let n = 1000003u64 * 1000033u64;
        let f = factorize(n);
        assert!(f.contains(&(1000003, 1)) && f.contains(&(1000033, 1)));
    }

    #[test]
    fn test_prime_number() {
        assert_eq!(factorize(104729), vec![(104729, 1)]);
    }

    #[test]
    fn test_pollards_rho_random() {
        for &n in &[999983 * 1000003, 123457 * 987653, 104729 * 999983] {
            let f = factorize(n);
            assert!(f.len() >= 2);
            assert_eq!(f.iter().map(|(p, c)| p.pow(*c)).product::<u64>(), n);
        }
    }
}
