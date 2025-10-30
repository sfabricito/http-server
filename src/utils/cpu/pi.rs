use num_bigint::BigInt;
use num_traits::{One, Zero};

/// Compute π using the Chudnovsky algorithm with arbitrary precision (BigInt only).
pub fn pi_number(digits: usize) -> String {
    let extra_digits = 2;
    let scale = BigInt::from(10u64).pow((digits + extra_digits) as u32);
    let mut sum = BigInt::zero();

    // constants as BigInt to prevent overflow
    let c = BigInt::from(640320u64);
    let c3_24 = c.pow(3) / BigInt::from(24u64);

    // sqrt(10005) * scale  (no u64 overflow)
    let sqrt_10005 = big_sqrt(&(BigInt::from(10005u64) * &scale * &scale));

    let n_terms = (digits as f64 / 14.0).ceil() as usize + 1;

    for k in 0..n_terms {
        let num = factorial(6 * k)
            * BigInt::from(13591409u64 + 545140134u64 * k as u64);
        let den = factorial(3 * k)
            * factorial(k).pow(3u32)
            * c3_24.pow(k as u32);

        let term = if k % 2 == 0 {
            num / &den
        } else {
            -num / &den
        };

        sum += term;
    }

    let factor = BigInt::from(426880u64) * sqrt_10005;
    let pi = (&factor * &scale) / sum;
    format_pi_string(&pi, digits)
}

/// Compute n! safely.
fn factorial(n: usize) -> BigInt {
    let mut result = BigInt::one();
    for i in 2..=n {
        result *= i;
    }
    result
}

/// Integer sqrt via Newton’s method.
fn big_sqrt(n: &BigInt) -> BigInt {
    if n.is_zero() {
        return BigInt::zero();
    }
    let two = BigInt::from(2u8);
    let mut x = n.clone();
    let mut y = (&x + n / &x) / &two;
    while y < x {
        x = y.clone();
        y = (&x + n / &x) / &two;
    }
    x
}

fn format_pi_string(pi: &BigInt, digits: usize) -> String {
    let mut pi_str = pi.to_str_radix(10);

    if pi_str.len() < 2 {
        pi_str.insert(0, '0');
    }

    if pi_str.len() > 1 {
        pi_str.insert(1, '.');
    } else {
        pi_str.push('.');
    }

    let max_len = 2 + digits; // "3." + N digits
    if pi_str.len() > max_len {
        pi_str.truncate(max_len);
    }

    pi_str
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pi_10() {
        let pi_10 = pi_number(10);
        assert!(pi_10.starts_with("3.14159"), "Got {}", pi_10);
    }

    #[test]
    fn test_pi_50() {
        let pi_50 = pi_number(50);
        assert!(
            pi_50.starts_with("3.14159265358979323846264338327950288419716939937510"),
            "Got {}",
            pi_50
        );
    }
}
