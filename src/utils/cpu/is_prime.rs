use crate::utils::math::{mul_mod_u64, pow_mod_u64};

#[derive(Copy, Clone)]
pub enum PrimeMethod { Trial, MillerRabin }

pub fn is_prime(n: u64, method: PrimeMethod) -> bool {
    if n < 2 { return false; }
    // Pre-compute prime numbers
    for p in [2u64,3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,73,79,83,89,97].iter() {
        if n == *p { return true; }
        if n % *p == 0 { return n == *p; }
    }
    match method {
        PrimeMethod::Trial => {
            // division trial until √n
            let mut d = 31u64;
            while d*d <= n {
                if n % d == 0 { return false; }
                d += 2;
            }
            true
        }
        PrimeMethod::MillerRabin => miller_rabin_u64(n),
    }
}

// Deterministic Miller–Rabin for u64 with known bases.
fn miller_rabin_u64(n: u64) -> bool {
    if n < 2 { return false; }
    // decompose n-1 = d * 2^s
    let mut d = n - 1;
    let mut s = 0u32;
    while d % 2 == 0 { d >>= 1; s += 1; }

    // Minimum safe set for u64:
    let bases: [u64; 7] = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];

    'outer: for &a in &bases {
        let a = a % n;
        if a == 0 { continue; }
        let mut x = pow_mod_u64(a, d, n);
        if x == 1 || x == n - 1 { continue 'outer; }
        for _ in 1..s {
            x = mul_mod_u64(x, x, n);
            if x == n - 1 { continue 'outer; }
        }
        return false;
    }
    true
}

/// Convenience wrapper for external callers and tests.
pub fn check(n: u64) -> bool {
    is_prime(n, PrimeMethod::MillerRabin)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_primes() {
        let primes = [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29];
        for &p in &primes {
            assert!(check(p), "{p} should be prime");
        }
    }

    #[test]
    fn test_small_composites() {
        let comps = [0u64, 1, 4, 6, 8, 9, 10, 12, 14, 15];
        for &c in &comps {
            assert!(!check(c), "{c} should not be prime");
        }
    }

    #[test]
    fn test_medium_primes() {
        let primes = [7919u64, 104729, 999983, 15485863];
        for &p in &primes {
            assert!(check(p), "{p} should be prime");
        }
    }

    #[test]
    fn test_large_primes() {
        let primes = [
            1_000_000_007u64,
            1_000_000_009,
            4_294_967_291,
            18_446_744_073_709_551_557, // 2^64 - 59
        ];
        for &p in &primes {
            assert!(check(p), "{p} should be prime");
        }
    }

    #[test]
    fn test_large_composites() {
        let comps = [
            1_000_000_000u64,
            4_294_967_296,
            9_223_372_036_854_775_808,
        ];
        for &c in &comps {
            assert!(!check(c), "{c} should not be prime");
        }
    }

    #[test]
    fn test_carmichael_numbers() {
        let carmichaels = [561u64, 1105, 1729, 2465, 6601, 8911, 10585, 15841];
        for &n in &carmichaels {
            assert!(!check(n), "{n} should not be prime (Carmichael number)");
        }
    }

    #[test]
    fn test_even_numbers() {
        for n in (4u64..100).step_by(2) {
            assert!(!check(n), "{n} is even and >2, should not be prime");
        }
    }

    #[test]
    fn test_boundary_values() {
        assert!(!check(0), "0 is not prime");
        assert!(!check(1), "1 is not prime");
        assert!(check(2), "2 is the smallest prime");
        assert!(check(u64::MAX - 58), "u64::MAX - 58 (prime near limit)");
        assert!(!check(u64::MAX), "u64::MAX should not be prime");
    }

    #[test]
    fn test_consistency_between_methods() {
        for n in 1u64..5000 {
            let trial = is_prime(n, PrimeMethod::Trial);
            let mr = is_prime(n, PrimeMethod::MillerRabin);
            assert_eq!(trial, mr, "Mismatch at n = {n}");
        }
    }
}
