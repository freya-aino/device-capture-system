//! Decode wire payloads into `Image`s. The wire carries the camera's native format (`MJPG`, `YUYV`);
//! frames that were already pre-processed carry `RGB3` or `GREY`.

use anyhow::{Context, Result, anyhow, bail, ensure};
use jpeg_decoder::{Decoder, PixelFormat};

use crate::image::{Image, ImageFormat};

pub fn decode_payload(data: &[u8], pixel_format: &str, frame_shape: &[u32]) -> Result<Image> {
    match pixel_format {
        "MJPG" => decode_mjpg(data),
        "YUYV" => {
            let (width, height) = shape(frame_shape)?;
            decode_yuyv(data, width, height)
        }
        "RGB3" => {
            let (width, height) = shape(frame_shape)?;
            Image::new(width, height, ImageFormat::Rgb, data.to_vec())
        }
        "GREY" => {
            let (width, height) = shape(frame_shape)?;
            Image::new(width, height, ImageFormat::Grey, data.to_vec())
        }
        other => bail!("unsupported pixel format {other:?}"),
    }
}

fn shape(frame_shape: &[u32]) -> Result<(u32, u32)> {
    ensure!(
        frame_shape.len() >= 2,
        "frame_shape must be [width, height]"
    );
    Ok((frame_shape[0], frame_shape[1]))
}

fn decode_mjpg(data: &[u8]) -> Result<Image> {
    let mut decoder = Decoder::new(data);
    let pixels = decoder
        .decode()
        .context("MJPG payload is not a decodable JPEG")?;
    let info = decoder
        .info()
        .ok_or_else(|| anyhow!("JPEG has no frame header"))?;
    let (width, height) = (info.width as u32, info.height as u32);
    match info.pixel_format {
        PixelFormat::RGB24 => Image::new(width, height, ImageFormat::Rgb, pixels),
        PixelFormat::L8 => Image::new(width, height, ImageFormat::Grey, pixels),
        other => bail!("unsupported JPEG pixel format {other:?}"),
    }
}

/// YUYV 4:2:2 (Y0 U Y1 V per two pixels) to RGB with BT.601 limited-range coefficients, the
/// conversion V4L2 webcams use.
fn decode_yuyv(data: &[u8], width: u32, height: u32) -> Result<Image> {
    ensure!(width % 2 == 0, "YUYV width must be even, got {width}");
    let expected = width as usize * height as usize * 2;
    ensure!(
        data.len() == expected,
        "YUYV payload is {} bytes, expected {expected}",
        data.len()
    );
    let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
    for quad in data.chunks_exact(4) {
        let (y0, u, y1, v) = (quad[0], quad[1], quad[2], quad[3]);
        rgb.extend_from_slice(&yuv_to_rgb(y0, u, v));
        rgb.extend_from_slice(&yuv_to_rgb(y1, u, v));
    }
    Image::new(width, height, ImageFormat::Rgb, rgb)
}

fn yuv_to_rgb(y: u8, u: u8, v: u8) -> [u8; 3] {
    let c = 298 * (y as i32 - 16);
    let d = u as i32 - 128;
    let e = v as i32 - 128;
    let clamp = |x: i32| ((x + 128) >> 8).clamp(0, 255) as u8;
    [
        clamp(c + 409 * e),
        clamp(c - 100 * d - 208 * e),
        clamp(c + 516 * d),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yuyv_neutral_chroma_maps_limited_range_to_full_range() {
        // Y=16 is black and Y=235 is white in limited range.
        let image = decode_payload(&[16, 128, 235, 128], "YUYV", &[2, 1]).unwrap();
        assert_eq!(image.data, vec![0, 0, 0, 255, 255, 255]);
    }

    #[test]
    fn yuyv_wrong_size_is_an_error() {
        assert!(decode_payload(&[0; 6], "YUYV", &[2, 1]).is_err());
    }

    #[test]
    fn uncompressed_formats_pass_through() {
        let rgb = decode_payload(&[1, 2, 3], "RGB3", &[1, 1]).unwrap();
        assert_eq!((rgb.format, rgb.data), (ImageFormat::Rgb, vec![1, 2, 3]));
        let grey = decode_payload(&[7, 8], "GREY", &[2, 1]).unwrap();
        assert_eq!(grey.format, ImageFormat::Grey);
    }

    #[test]
    fn unknown_format_and_bad_jpeg_are_errors() {
        assert!(decode_payload(&[], "H264", &[1, 1]).is_err());
        assert!(decode_payload(b"not a jpeg", "MJPG", &[1, 1]).is_err());
    }
}
