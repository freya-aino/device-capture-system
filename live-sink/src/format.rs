use anyhow::{Result, bail, ensure};

/// The pixel format the loopback device is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Mjpg,
    Yuyv,
}

impl OutputFormat {
    pub fn fourcc(self) -> &'static str {
        match self {
            OutputFormat::Mjpg => "MJPG",
            OutputFormat::Yuyv => "YUYV",
        }
    }
}

/// What the sink does to each frame before writing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversion {
    Passthrough,
    RgbToYuyv,
    GreyToYuyv,
}

/// The output decided from the first frame. Every later frame must match `input_format` and
/// `width`/`height`, or the sink stops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPlan {
    pub input_format: String,
    pub width: u32,
    pub height: u32,
    pub output: OutputFormat,
    pub conversion: Conversion,
}

/// Chooses the loopback format for a stream: `MJPG` and `YUYV` pass through, `RGB3` and `GREY` are
/// converted to `YUYV`, which every V4L2 client reads.
pub fn plan_output(pixel_format: &str, frame_shape: &[u32]) -> Result<OutputPlan> {
    ensure!(
        frame_shape.len() >= 2,
        "frame_shape must be [width, height]"
    );
    let (width, height) = (frame_shape[0], frame_shape[1]);
    ensure!(
        width > 0 && height > 0,
        "frame size {width}x{height} is empty"
    );
    let (output, conversion) = match pixel_format {
        "MJPG" => (OutputFormat::Mjpg, Conversion::Passthrough),
        "YUYV" => (OutputFormat::Yuyv, Conversion::Passthrough),
        "RGB3" => (OutputFormat::Yuyv, Conversion::RgbToYuyv),
        "GREY" => (OutputFormat::Yuyv, Conversion::GreyToYuyv),
        other => bail!("cannot forward pixel format {other:?} to a V4L2 device"),
    };
    if conversion != Conversion::Passthrough {
        ensure!(
            width % 2 == 0,
            "YUYV needs an even width, the stream is {width} wide"
        );
    }
    Ok(OutputPlan {
        input_format: pixel_format.to_string(),
        width,
        height,
        output,
        conversion,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_formats_pass_through() {
        let plan = plan_output("MJPG", &[1920, 1080]).unwrap();
        assert_eq!(
            (plan.output, plan.conversion),
            (OutputFormat::Mjpg, Conversion::Passthrough)
        );
        let plan = plan_output("YUYV", &[640, 480]).unwrap();
        assert_eq!(
            (plan.output, plan.conversion),
            (OutputFormat::Yuyv, Conversion::Passthrough)
        );
    }

    #[test]
    fn processed_formats_become_yuyv() {
        assert_eq!(
            plan_output("RGB3", &[1080, 1920]).unwrap().conversion,
            Conversion::RgbToYuyv
        );
        assert_eq!(
            plan_output("GREY", &[320, 240]).unwrap().conversion,
            Conversion::GreyToYuyv
        );
    }

    #[test]
    fn unsupported_inputs_are_errors() {
        assert!(plan_output("H264", &[640, 480]).is_err());
        assert!(
            plan_output("RGB3", &[641, 480]).is_err(),
            "odd width cannot be YUYV"
        );
        assert!(plan_output("YUYV", &[640]).is_err());
        assert!(plan_output("GREY", &[0, 480]).is_err());
    }
}
