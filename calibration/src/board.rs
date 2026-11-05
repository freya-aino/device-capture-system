//! The calibration target: one ChArUco board shared by the detector, the fixture and the tests.

use anyhow::Result;
use opencv::core::{Size, no_array};
use opencv::objdetect::{CharucoBoard, PredefinedDictionaryType, get_predefined_dictionary};

pub const SQUARES_X: i32 = 7;
pub const SQUARES_Y: i32 = 5;
/// Metres.
pub const SQUARE_LENGTH: f32 = 0.04;
/// Metres.
pub const MARKER_LENGTH: f32 = 0.03;

/// Inner chessboard corners are the reference points: `(SQUARES_X - 1) * (SQUARES_Y - 1)` of them.
pub const CORNER_COUNT: usize = ((SQUARES_X - 1) * (SQUARES_Y - 1)) as usize;

pub fn default_board() -> Result<CharucoBoard> {
    let dictionary = get_predefined_dictionary(PredefinedDictionaryType::DICT_5X5_100)?;
    Ok(CharucoBoard::new(
        Size::new(SQUARES_X, SQUARES_Y),
        SQUARE_LENGTH,
        MARKER_LENGTH,
        &dictionary,
        &no_array(),
    )?)
}
