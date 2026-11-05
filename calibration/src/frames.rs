//! Decode camera payloads into BGR images.
//!
//! The wire carries the camera's native format (`pixel_format`): `MJPG` is one JPEG per frame, `YUYV`
//! is two bytes per pixel. Request an uncompressed mode with `DEVICE_NODE_CAMERA_WIDTH=640 ...` to
//! avoid the JPEG decode.

use anyhow::{Result, bail, ensure};
use opencv::core::{CV_8UC2, Mat, Vector};
use opencv::imgcodecs::{IMREAD_COLOR, imdecode};
use opencv::imgproc::{COLOR_YUV2BGR_YUY2, cvt_color_def};
use opencv::prelude::*;
use shared::FramePacket;

pub fn decode_camera_frame(packet: &FramePacket) -> Result<Mat> {
    let info = &packet.frame_info;
    let Some(format) = info.pixel_format.as_deref() else {
        bail!("frame carries no pixel format; is it from a camera?");
    };
    decode_payload(&packet.data, format, &info.frame_shape)
}

pub fn decode_payload(data: &[u8], pixel_format: &str, frame_shape: &[u32]) -> Result<Mat> {
    match pixel_format {
        "MJPG" => {
            let image = imdecode(&Vector::<u8>::from_slice(data), IMREAD_COLOR)?;
            ensure!(!image.empty(), "MJPG payload is not a decodable JPEG");
            Ok(image)
        }
        "YUYV" => {
            ensure!(frame_shape.len() >= 2, "frame_shape must be [width, height]");
            let (width, height) = (frame_shape[0] as i32, frame_shape[1] as i32);
            let expected = width as usize * height as usize * 2;
            ensure!(
                data.len() == expected,
                "YUYV payload is {} bytes, expected {expected}",
                data.len()
            );
            let mut yuyv = Mat::new_rows_cols_with_default(height, width, CV_8UC2, 0.into())?;
            yuyv.data_bytes_mut()?.copy_from_slice(data);
            let mut bgr = Mat::default();
            cvt_color_def(&yuyv, &mut bgr, COLOR_YUV2BGR_YUY2)?;
            Ok(bgr)
        }
        other => bail!("unsupported pixel format {other:?}"),
    }
}
