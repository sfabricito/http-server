use std::fs::File;
use std::io::Write;
use std::time::Instant;

/// Generate a Mandelbrot iteration map as a 2D Vec<Vec<u32>>.
/// Optionally, write a PGM/PPM file for visualization.
pub fn mandelbrot(
    width: usize,
    height: usize,
    max_iter: u32,
    dump_filename: Option<&str>,
) -> (Vec<Vec<u32>>, u128) {
    let start = Instant::now();

    let mut data = vec![vec![0u32; width]; height];

    // Viewport boundaries
    let xmin = -2.5;
    let xmax = 1.0;
    let ymin = -1.25;
    let ymax = 1.25;

    for y in 0..height {
        let cy = ymin + (y as f64 / height as f64) * (ymax - ymin);
        for x in 0..width {
            let cx = xmin + (x as f64 / width as f64) * (xmax - xmin);
            let mut zx = 0.0;
            let mut zy = 0.0;
            let mut iter = 0;

            while zx * zx + zy * zy <= 4.0 && iter < max_iter {
                let xtemp = zx * zx - zy * zy + cx;
                zy = 2.0 * zx * zy + cy;
                zx = xtemp;
                iter += 1;
            }

            data[y][x] = iter;
        }
    }

    // Optional: write to disk
    if let Some(filename) = dump_filename {
        if filename.ends_with(".pgm") {
            let mut f = File::create(filename).expect("Failed to create PGM file");
            writeln!(f, "P2\n{} {}\n{}", width, height, max_iter).unwrap();
            for row in &data {
                for val in row {
                    write!(f, "{} ", val).unwrap();
                }
                writeln!(f).unwrap();
            }
        } else if filename.ends_with(".ppm") {
            let mut f = File::create(filename).expect("Failed to create PPM file");
            writeln!(f, "P3\n{} {}\n{}", width, height, 255).unwrap();
            for row in &data {
                for val in row {
                    let color = ((*val as f64 / max_iter as f64) * 255.0) as u8;
                    writeln!(f, "{} {} {} ", color, 0, 255 - color).unwrap();
                }
            }
        }
    }

    (data, start.elapsed().as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_mandelbrot_small_matrix() {
        let (map, elapsed) = mandelbrot(5, 5, 10, None);
        assert_eq!(map.len(), 5);
        assert_eq!(map[0].len(), 5);
        assert!(elapsed < 2000);
    }

    #[test]
    fn test_mandelbrot_intensity_range() {
        let (map, _) = mandelbrot(20, 20, 50, None);
        let mut max_seen = 0;
        for row in &map {
            for &v in row {
                assert!(v <= 50);
                if v > max_seen {
                    max_seen = v;
                }
            }
        }
        assert!(max_seen > 0);
    }

    #[test]
    fn test_mandelbrot_pgm_dump() {
        let filename = "test_mandelbrot.pgm";
        let (_map, _) = mandelbrot(10, 10, 30, Some(filename));
        let content = fs::read_to_string(filename).unwrap();
        assert!(content.contains("P2"));
        fs::remove_file(filename).unwrap();
    }

    #[test]
    fn test_mandelbrot_ppm_dump() {
        let filename = "test_mandelbrot.ppm";
        let (_map, _) = mandelbrot(10, 10, 30, Some(filename));
        let content = fs::read_to_string(filename).unwrap();
        assert!(content.contains("P3"));
        fs::remove_file(filename).unwrap();
    }
}
