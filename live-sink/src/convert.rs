//! Conversion of the hub's uncompressed formats to `YUYV`, the format every V4L2 client reads.
//! BT.601 limited range, the inverse of `preprocess`'s YUYV decode.

use anyhow::{Result, bail, ensure};

use crate::{Conversion, OutputPlan};

/// Limited-range luma of an RGB pixel.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    (16 + ((66 * r + 129 * g + 25 * b + 128) >> 8)).clamp(16, 235) as u8
}

/// Limited-range chroma (U, V) of an RGB pixel.
fn chroma(r: u8, g: u8, b: u8) -> (u8, u8) {
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    let u = 128 + ((-38 * r - 74 * g + 112 * b + 128) >> 8);
    let v = 128 + ((112 * r - 94 * g - 18 * b + 128) >> 8);
    (u.clamp(16, 240) as u8, v.clamp(16, 240) as u8)
}

/// Full-range grey level to limited-range luma.
fn grey_luma(g: u8) -> u8 {
    (16 + (u32::from(g) * 219 + 127) / 255) as u8
}

/// Packs `RGB3` pixels as `YUYV`; the chroma of each pixel pair is the chroma of their mean colour.
pub fn rgb_to_yuyv(rgb: &[u8], width: u32, height: u32, out: &mut Vec<u8>) -> Result<()> {
    let pixels = width as usize * height as usize;
    ensure!(width % 2 == 0, "YUYV needs an even width, got {width}");
    ensure!(
        rgb.len() == pixels * 3,
        "RGB3 payload is {} bytes, expected {}",
        rgb.len(),
        pixels * 3
    );
    out.clear();
    out.reserve(pixels * 2);
    for pair in rgb.chunks_exact(6) {
        let (r0, g0, b0, r1, g1, b1) = (pair[0], pair[1], pair[2], pair[3], pair[4], pair[5]);
        let mean = |a: u8, b: u8| ((u16::from(a) + u16::from(b) + 1) / 2) as u8;
        let (u, v) = chroma(mean(r0, r1), mean(g0, g1), mean(b0, b1));
        out.extend_from_slice(&[luma(r0, g0, b0), u, luma(r1, g1, b1), v]);
    }
    Ok(())
}

/// Packs `GREY` pixels as `YUYV` with neutral chroma.
pub fn grey_to_yuyv(grey: &[u8], width: u32, height: u32, out: &mut Vec<u8>) -> Result<()> {
    let pixels = width as usize * height as usize;
    ensure!(width % 2 == 0, "YUYV needs an even width, got {width}");
    ensure!(
        grey.len() == pixels,
        "GREY payload is {} bytes, expected {pixels}",
        grey.len()
    );
    out.clear();
    out.reserve(pixels * 2);
    for pair in grey.chunks_exact(2) {
        out.extend_from_slice(&[grey_luma(pair[0]), 128, grey_luma(pair[1]), 128]);
    }
    Ok(())
}

/// The bytes to write to the device for one frame of `plan`: the payload itself for a passthrough
/// format (after a size check), or the converted frame in `buffer`.
pub fn convert_frame<'a>(
    plan: &OutputPlan,
    data: &'a [u8],
    buffer: &'a mut Vec<u8>,
) -> Result<&'a [u8]> {
    let pixels = plan.width as usize * plan.height as usize;
    match plan.conversion {
        Conversion::Passthrough => {
            match plan.input_format.as_str() {
                "YUYV" => ensure!(
                    data.len() == pixels * 2,
                    "YUYV payload is {} bytes, expected {}",
                    data.len(),
                    pixels * 2
                ),
                "MJPG" => ensure!(!data.is_empty(), "empty MJPG payload"),
                other => bail!("no passthrough for {other:?}"),
            }
            Ok(data)
        }
        Conversion::RgbToYuyv => {
            rgb_to_yuyv(data, plan.width, plan.height, buffer)?;
            Ok(buffer.as_slice())
        }
        Conversion::GreyToYuyv => {
            grey_to_yuyv(data, plan.width, plan.height, buffer)?;
            Ok(buffer.as_slice())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_output;
    use preprocess::decode_payload;

    #[test]
    fn exact_pixels_for_black_white_and_primaries() {
        let mut out = Vec::new();
        rgb_to_yuyv(&[0, 0, 0, 0, 0, 0], 2, 1, &mut out).unwrap();
        assert_eq!(out, [16, 128, 16, 128]);
        rgb_to_yuyv(&[255, 255, 255, 255, 255, 255], 2, 1, &mut out).unwrap();
        assert_eq!(out, [235, 128, 235, 128]);
        rgb_to_yuyv(&[255, 0, 0, 255, 0, 0], 2, 1, &mut out).unwrap();
        assert_eq!(out, [82, 90, 82, 240]);
        grey_to_yuyv(&[0, 255], 2, 1, &mut out).unwrap();
        assert_eq!(out, [16, 128, 235, 128]);
        grey_to_yuyv(&[128, 128], 2, 1, &mut out).unwrap();
        assert_eq!(out, [126, 128, 126, 128]);
    }

    #[test]
    fn round_trip_through_the_preprocess_decoder_is_within_two_levels() {
        let colours: [[u8; 3]; 8] = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [200, 100, 50],
            [17, 230, 99],
            [128, 128, 128],
            [250, 250, 5],
            [3, 60, 180],
        ];
        // Pairs of identical pixels: chroma subsampling then loses nothing.
        let rgb: Vec<u8> = colours
            .iter()
            .flat_map(|c| [c, c])
            .flatten()
            .copied()
            .collect();
        let mut yuyv = Vec::new();
        rgb_to_yuyv(&rgb, 16, 1, &mut yuyv).unwrap();
        let back = decode_payload(&yuyv, "YUYV", &[16, 1]).unwrap();
        for (a, b) in rgb.iter().zip(&back.data) {
            assert!((i16::from(*a) - i16::from(*b)).abs() <= 2, "{a} -> {b}");
        }
        let grey: Vec<u8> = (0..=255u8).step_by(5).collect();
        let n = grey.len() as u32;
        grey_to_yuyv(&grey, n, 1, &mut yuyv).unwrap();
        let back = decode_payload(&yuyv, "YUYV", &[n, 1]).unwrap();
        for (g, px) in grey.iter().zip(back.data.chunks_exact(3)) {
            for c in px {
                assert!((i16::from(*g) - i16::from(*c)).abs() <= 2, "{g} -> {px:?}");
            }
        }
    }

    #[test]
    fn convert_frame_checks_sizes_and_passes_native_formats_through() {
        let mut buf = Vec::new();
        let yuyv = plan_output("YUYV", &[2, 1]).unwrap();
        assert_eq!(
            convert_frame(&yuyv, &[1, 2, 3, 4], &mut buf).unwrap(),
            &[1, 2, 3, 4]
        );
        assert!(convert_frame(&yuyv, &[1, 2, 3], &mut buf).is_err());
        let mjpg = plan_output("MJPG", &[2, 1]).unwrap();
        assert_eq!(
            convert_frame(&mjpg, &[0xff, 0xd8], &mut buf).unwrap(),
            &[0xff, 0xd8]
        );
        assert!(convert_frame(&mjpg, &[], &mut buf).is_err());
        let rgb = plan_output("RGB3", &[2, 1]).unwrap();
        assert_eq!(
            convert_frame(&rgb, &[0; 6], &mut buf).unwrap(),
            &[16, 128, 16, 128]
        );
        assert!(convert_frame(&rgb, &[0; 5], &mut buf).is_err());
        let grey = plan_output("GREY", &[2, 1]).unwrap();
        assert_eq!(
            convert_frame(&grey, &[255, 0], &mut buf).unwrap(),
            &[235, 128, 16, 128]
        );
    }
}
