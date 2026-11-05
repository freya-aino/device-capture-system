//! Regenerates tests/fixtures/charuco.png. The checked-in PNG is the test input; run this only if
//! the board definition in src/board.rs changes:
//! `cargo run -p calibration --example make-fixture`

use anyhow::Result;
use calibration::default_board;
use opencv::core::{Mat, Size, Vector};
use opencv::imgcodecs::imwrite;
use opencv::prelude::*;

const SQUARE_PX: i32 = 100;
const MARGIN_PX: i32 = 50;

fn main() -> Result<()> {
    let size = Size::new(7 * SQUARE_PX + 2 * MARGIN_PX, 5 * SQUARE_PX + 2 * MARGIN_PX);
    let mut image = Mat::default();
    default_board()?.generate_image(size, &mut image, MARGIN_PX, 1)?;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/charuco.png");
    imwrite(path, &image, &Vector::new())?;
    println!("{path} {}x{}", size.width, size.height);
    Ok(())
}
