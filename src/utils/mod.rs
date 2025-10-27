
pub mod text;
pub mod math;
pub mod file;
pub mod time;
pub mod hash;
pub mod commands;

// cpu intensive utilities
pub mod cpu {
    pub mod is_prime;
    pub mod factor;
    pub mod mandelbrot;
    pub mod matrixmul;
}

pub mod io {
    pub mod sortfile;
    pub mod wordcount;
    pub mod grep;
    pub mod compress;
    pub mod hash_file;
}