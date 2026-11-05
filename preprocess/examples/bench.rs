//! Times decode and each transform on one image file:
//! `cargo run --release -p preprocess --example bench -- frame.jpg`

use std::time::Instant;

use anyhow::{Context, Result};
use preprocess::{FlipAxis, Rotation, Transform, decode_payload};

fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("usage: bench <file.jpg>")?;
    let data = std::fs::read(&path)?;
    let runs = 20;

    let start = Instant::now();
    let mut image = decode_payload(&data, "MJPG", &[0, 0])?;
    for _ in 1..runs {
        image = decode_payload(&data, "MJPG", &[0, 0])?;
    }
    println!(
        "decode {}x{}: {:.1} ms",
        image.width,
        image.height,
        ms(start, runs)
    );

    for t in [
        Transform::Rotate(Rotation::Deg90),
        Transform::Rotate(Rotation::Deg180),
        Transform::Flip(FlipAxis::Horizontal),
        Transform::Crop {
            x: 0,
            y: 0,
            width: image.width / 2,
            height: image.height / 2,
        },
        Transform::Resize {
            width: image.width / 2,
            height: image.height / 2,
        },
        Transform::Brightness(10),
        Transform::Contrast(1.2),
        Transform::Gamma(0.8),
        Transform::Grayscale,
    ] {
        let start = Instant::now();
        for _ in 0..runs {
            std::hint::black_box(t.apply(image.clone())?);
        }
        println!("{t:?}: {:.1} ms", ms(start, runs));
    }
    let start = Instant::now();
    for _ in 0..runs {
        std::hint::black_box(image.clone());
    }
    println!("(clone, included above): {:.1} ms", ms(start, runs));
    Ok(())
}

fn ms(start: Instant, runs: u32) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0 / runs as f64
}
