use anyhow::{Result, ensure};

/// Pixel layout of a decoded image. The FourCC is what goes into `pixel_format` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// 24-bit RGB, 3 bytes per pixel (FourCC `RGB3`).
    Rgb,
    /// 8-bit luma, 1 byte per pixel (FourCC `GREY`).
    Grey,
}

impl ImageFormat {
    pub fn channels(self) -> usize {
        match self {
            ImageFormat::Rgb => 3,
            ImageFormat::Grey => 1,
        }
    }

    pub fn fourcc(self) -> &'static str {
        match self {
            ImageFormat::Rgb => "RGB3",
            ImageFormat::Grey => "GREY",
        }
    }
}

/// A decoded, uncompressed image, rows top to bottom, pixels left to right.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub data: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32, format: ImageFormat, data: Vec<u8>) -> Result<Self> {
        let expected = width as usize * height as usize * format.channels();
        ensure!(
            data.len() == expected,
            "{}x{} {} image needs {expected} bytes, got {}",
            width,
            height,
            format.fourcc(),
            data.len()
        );
        Ok(Image {
            width,
            height,
            format,
            data,
        })
    }

    /// The bytes of the pixel at (x, y).
    pub fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let c = self.format.channels();
        let i = (y as usize * self.width as usize + x as usize) * c;
        &self.data[i..i + c]
    }
}
