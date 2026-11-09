//! The `ffmpeg` child process that writes a camera stream, and the command line it gets.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};

use crate::Codec;

/// `ffmpeg` is a runtime dependency of the recorder only. Checked at start.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// What a camera's video file is encoded with: `copy` for MJPG input, the configured codec
/// otherwise.
pub fn codec_name(pixel_format: &str, codec: Codec) -> &'static str {
    match (pixel_format, codec) {
        ("MJPG", _) => "copy",
        (_, Codec::Ffv1) => "ffv1",
        (_, Codec::H264) => "h264",
    }
}

/// The `ffmpeg` command line for one camera: MJPG is demuxed and stream-copied, uncompressed
/// formats are read as `rawvideo` and encoded. `fps` is the nominal rate for players only.
pub fn ffmpeg_args(
    pixel_format: &str,
    width: u32,
    height: u32,
    fps: f64,
    codec: Codec,
    output: &Path,
) -> Result<Vec<String>> {
    let mut args: Vec<String> = ["-y", "-nostdin", "-loglevel", "error"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let framerate = format!("{fps:.3}");
    match pixel_format {
        "MJPG" => {
            args.extend(
                [
                    "-f",
                    "mjpeg",
                    "-framerate",
                    &framerate,
                    "-i",
                    "pipe:0",
                    "-c:v",
                    "copy",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
        }
        other => {
            let pix_fmt = match other {
                "YUYV" => "yuyv422",
                "RGB3" => "rgb24",
                "GREY" => "gray",
                unknown => bail!("cannot record pixel format {unknown:?}"),
            };
            let size = format!("{width}x{height}");
            args.extend(
                [
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    pix_fmt,
                    "-video_size",
                    &size,
                    "-framerate",
                    &framerate,
                    "-i",
                    "pipe:0",
                ]
                .iter()
                .map(|s| s.to_string()),
            );
            match codec {
                Codec::Ffv1 => args.extend(["-c:v", "ffv1"].iter().map(|s| s.to_string())),
                Codec::H264 => args.extend(
                    [
                        "-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p",
                    ]
                    .iter()
                    .map(|s| s.to_string()),
                ),
            }
        }
    }
    args.push(output.to_string_lossy().into_owned());
    Ok(args)
}

/// A running `ffmpeg` fed through stdin.
pub struct FfmpegWriter {
    child: Child,
    output: PathBuf,
}

impl FfmpegWriter {
    pub fn spawn(args: &[String], output: &Path) -> Result<Self> {
        let child = Command::new("ffmpeg")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to start ffmpeg")?;
        Ok(FfmpegWriter {
            child,
            output: output.to_path_buf(),
        })
    }

    pub fn write_frame(&mut self, data: &[u8]) -> Result<()> {
        let stdin = self
            .child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("ffmpeg stdin is closed"))?;
        stdin.write_all(data).with_context(|| {
            format!(
                "ffmpeg stopped accepting frames for {}",
                self.output.display()
            )
        })
    }

    /// Closes stdin and waits for `ffmpeg` to finish the file.
    pub fn finish(mut self) -> Result<()> {
        drop(self.child.stdin.take());
        let out = self
            .child
            .wait_with_output()
            .context("waiting for ffmpeg")?;
        if !out.status.success() {
            bail!(
                "ffmpeg exited with {} writing {}: {}",
                out.status,
                self.output.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mjpg_is_stream_copied() {
        let args =
            ffmpeg_args("MJPG", 1920, 1080, 30.0, Codec::Ffv1, Path::new("out.mkv")).unwrap();
        let line = args.join(" ");
        assert!(
            line.contains("-f mjpeg -framerate 30.000 -i pipe:0 -c:v copy out.mkv"),
            "{line}"
        );
        assert!(!line.contains("rawvideo"));
        assert_eq!(codec_name("MJPG", Codec::H264), "copy");
    }

    #[test]
    fn uncompressed_input_is_rawvideo_with_the_configured_codec() {
        let args = ffmpeg_args("YUYV", 640, 480, 29.97, Codec::Ffv1, Path::new("v.mkv")).unwrap();
        let line = args.join(" ");
        assert!(
            line.contains("-f rawvideo -pix_fmt yuyv422 -video_size 640x480 -framerate 29.970 -i pipe:0 -c:v ffv1"),
            "{line}"
        );
        let args = ffmpeg_args("RGB3", 8, 8, 30.0, Codec::H264, Path::new("v.mkv")).unwrap();
        assert!(args.join(" ").contains("-pix_fmt rgb24"));
        assert!(args.join(" ").contains("-c:v libx264"));
        assert_eq!(codec_name("GREY", Codec::H264), "h264");
        assert_eq!(codec_name("YUYV", Codec::Ffv1), "ffv1");
    }

    #[test]
    fn unknown_pixel_formats_are_errors() {
        assert!(ffmpeg_args("H264", 1, 1, 30.0, Codec::Ffv1, Path::new("x")).is_err());
    }
}
