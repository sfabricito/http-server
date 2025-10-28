use sha2::{Digest, Sha256};
use std::time::Instant;

/// Prime modulus: keeps values bounded & fully deterministic across platforms.
const MOD: u64 = 1_000_000_007;

/// XorShift64* PRNG (deterministic). If seed == 0, use a golden-ratio constant to avoid lock-up.
#[inline]
fn xorshift64star_stream(seed: u64, count: usize) -> Vec<u64> {
    let mut x = if seed == 0 { 0x9e3779b97f4a7c15 } else { seed };
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x = x.wrapping_mul(0x2545F4914F6CDD1D);
        out.push(x);
    }
    out
}

/// Generate an N×N matrix (row-major) with entries in [0, MOD)
#[inline]
fn gen_matrix(n: usize, seed: u64) -> Vec<u64> {
    let raw = xorshift64star_stream(seed, n * n);
    raw.into_iter().map(|v| v % MOD).collect()
}

/// Multiply C = A × B (mod MOD), A,B row-major N×N. Cache-friendly using Bᵀ.
fn matmul_mod(a: &[u64], b: &[u64], n: usize) -> Vec<u64> {
    let mut c = vec![0u64; n * n];
    // Precompute Bᵀ to improve locality
    let mut bt = vec![0u64; n * n];
    for i in 0..n {
        for j in 0..n {
            bt[j * n + i] = b[i * n + j];
        }
    }
    for i in 0..n {
        let row = &a[i * n..(i + 1) * n];
        for j in 0..n {
            let col = &bt[j * n..(j + 1) * n];
            let mut s: u128 = 0;
            // Accumulate in u128 to avoid intermediate overflow before taking mod
            for k in 0..n {
                s += (row[k] as u128) * (col[k] as u128);
            }
            c[i * n + j] = (s % (MOD as u128)) as u64;
        }
    }
    c
}

/// Serialize a matrix (row-major) as big-endian u64 bytes (canonical for hashing).
fn to_be_bytes_u64_row_major(m: &[u64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(m.len() * 8);
    for &v in m {
        out.extend_from_slice(&v.to_be_bytes());
    }
    out
}

pub fn matrixmul(size: usize, seed: u64) -> (String, u128) {
    let start = Instant::now();

    // 1) matrices
    let a = gen_matrix(size, seed);
    let b = gen_matrix(size, seed ^ 0xDEADBEEFCAFEBABE);

    // 2) multiply (mod MOD)
    let c = matmul_mod(&a, &b, size);

    // 3) hash
    let bytes = to_be_bytes_u64_row_major(&c);
    let digest = Sha256::digest(&bytes);
    let hex = format!("{:x}", digest);

    (hex, start.elapsed().as_millis())
}

/// Convenience for HTTP handler: returns JSON string body { "n":..., "elapsed_ms":..., "sha256":"..." }
pub fn matrixmul_json(size: usize, seed: u64) -> String {
    let (hex, elapsed) = matrixmul(size, seed);
    format!(r#"{{"n":{},"elapsed_ms":{},"sha256":"{}"}}"#, size, elapsed, hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinism_same_seed_same_hash() {
        let (h1, _) = matrixmul(5, 42);
        let (h2, _) = matrixmul(5, 42);
        assert_eq!(h1, h2, "Same size & seed must yield identical hash");
    }

    #[test]
    fn seed_changes_hash() {
        let (h1, _) = matrixmul(5, 42);
        let (h2, _) = matrixmul(5, 43);
        assert_ne!(h1, h2, "Different seeds should produce different hashes");
    }

    #[test]
    fn small_regression_vectors() {
        // Precomputed with this implementation (mod=1_000_000_007, xorshift64* above, BE u64 serialization)
        let cases = [
            (1usize, 1u64, "19ce5379c7712f0d4d4d5f338604ded9a741e8b1ec1bd49c97d8803961cc20d6"),
            (2, 1,         "f450a1ab3c145c3c7ee5134e03dc0cbf23d675ac58226e61b2e83298d957d69c"),
            (2, 12345,     "f81dfa4519a60485fcccb7c73ea5bdab8423921f07882bee45b715e3b8fae56c"),
            (3, 12345,     "37eac5d5090c2ecd8c26c520d442859534affac295dc19393616d186d089fafe"),
            (5, 42,        "fe4cff7eda75bcb82c11c2728c4c867e4b7287138b1b1ee54b1315170af84ec0"),
        ];
        for (n, seed, expected) in cases {
            let (h, _) = matrixmul(n, seed);
            assert_eq!(h, expected, "Mismatch for n={n}, seed={seed}");
        }
    }

    #[test]
    fn performance_sanity_medium() {
        // Not a hard guarantee, just a sanity check (~O(N^3)). Adjust if CI is slow.
        let (_h, ms) = matrixmul(64, 7);
        assert!(ms < 3000, "64x64 should typically finish <3s, got {} ms", ms);
    }
}
