use anyhow::{Result, ensure};

use crate::image::{Image, ImageFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    /// Clockwise.
    Deg90,
    Deg180,
    Deg270,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipAxis {
    /// Mirror left to right.
    Horizontal,
    /// Mirror top to bottom.
    Vertical,
}

/// One basic image transform. Build them through `PreprocessConfig`, which validates parameters.
#[derive(Debug, Clone, PartialEq)]
pub enum Transform {
    Rotate(Rotation),
    Flip(FlipAxis),
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    /// Bilinear.
    Resize {
        width: u32,
        height: u32,
    },
    /// Added to every channel, saturating.
    Brightness(i16),
    /// Scales every channel around mid-grey (128).
    Contrast(f32),
    /// `out = 255 * (in / 255) ^ gamma`; below 1 brightens, above 1 darkens.
    Gamma(f32),
    /// BT.601 luma; the output is `GREY`.
    Grayscale,
}

impl Transform {
    pub fn apply(&self, image: Image) -> Result<Image> {
        match *self {
            Transform::Rotate(rotation) => Ok(rotate(&image, rotation)),
            Transform::Flip(axis) => Ok(flip(&image, axis)),
            Transform::Crop {
                x,
                y,
                width,
                height,
            } => crop(&image, x, y, width, height),
            Transform::Resize { width, height } => Ok(resize(&image, width, height)),
            Transform::Brightness(offset) => Ok(map_values(image, |v| {
                (v as i16 + offset).clamp(0, 255) as u8
            })),
            Transform::Contrast(factor) => Ok(map_values(image, |v| {
                ((v as f32 - 128.0) * factor + 128.0)
                    .round()
                    .clamp(0.0, 255.0) as u8
            })),
            Transform::Gamma(gamma) => {
                let lut: Vec<u8> = (0..=255u16)
                    .map(|v| (255.0 * (v as f32 / 255.0).powf(gamma)).round() as u8)
                    .collect();
                Ok(map_values(image, |v| lut[v as usize]))
            }
            Transform::Grayscale => Ok(grayscale(image)),
        }
    }
}

/// An ordered list of transforms for one device.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pipeline {
    pub steps: Vec<Transform>,
}

impl Pipeline {
    pub fn apply(&self, mut image: Image) -> Result<Image> {
        for step in &self.steps {
            image = step.apply(image)?;
        }
        Ok(image)
    }
}

fn remap(image: &Image, width: u32, height: u32, source: impl Fn(u32, u32) -> (u32, u32)) -> Image {
    let c = image.format.channels();
    let mut data = Vec::with_capacity(width as usize * height as usize * c);
    for y in 0..height {
        for x in 0..width {
            let (sx, sy) = source(x, y);
            data.extend_from_slice(image.pixel(sx, sy));
        }
    }
    Image {
        width,
        height,
        format: image.format,
        data,
    }
}

fn rotate(image: &Image, rotation: Rotation) -> Image {
    let (w, h) = (image.width, image.height);
    match rotation {
        Rotation::Deg90 => remap(image, h, w, |x, y| (y, h - 1 - x)),
        Rotation::Deg180 => remap(image, w, h, |x, y| (w - 1 - x, h - 1 - y)),
        Rotation::Deg270 => remap(image, h, w, |x, y| (w - 1 - y, x)),
    }
}

fn flip(image: &Image, axis: FlipAxis) -> Image {
    let (w, h) = (image.width, image.height);
    match axis {
        FlipAxis::Horizontal => remap(image, w, h, |x, y| (w - 1 - x, y)),
        FlipAxis::Vertical => remap(image, w, h, |x, y| (x, h - 1 - y)),
    }
}

fn crop(image: &Image, x: u32, y: u32, width: u32, height: u32) -> Result<Image> {
    ensure!(
        x as u64 + width as u64 <= image.width as u64
            && y as u64 + height as u64 <= image.height as u64,
        "crop {width}x{height} at ({x}, {y}) does not fit a {}x{} image",
        image.width,
        image.height
    );
    Ok(remap(image, width, height, |cx, cy| (x + cx, y + cy)))
}

/// Bilinear resize with pixel centres aligned (the OpenCV `INTER_LINEAR` convention).
fn resize(image: &Image, width: u32, height: u32) -> Image {
    let c = image.format.channels();
    let sx = image.width as f32 / width as f32;
    let sy = image.height as f32 / height as f32;
    let max_x = image.width - 1;
    let max_y = image.height - 1;
    let mut data = Vec::with_capacity(width as usize * height as usize * c);
    for y in 0..height {
        let fy = ((y as f32 + 0.5) * sy - 0.5).clamp(0.0, max_y as f32);
        let (y0, ty) = (fy.floor() as u32, fy.fract());
        let y1 = (y0 + 1).min(max_y);
        for x in 0..width {
            let fx = ((x as f32 + 0.5) * sx - 0.5).clamp(0.0, max_x as f32);
            let (x0, tx) = (fx.floor() as u32, fx.fract());
            let x1 = (x0 + 1).min(max_x);
            let (a, b) = (image.pixel(x0, y0), image.pixel(x1, y0));
            let (d, e) = (image.pixel(x0, y1), image.pixel(x1, y1));
            for ch in 0..c {
                let top = a[ch] as f32 * (1.0 - tx) + b[ch] as f32 * tx;
                let bottom = d[ch] as f32 * (1.0 - tx) + e[ch] as f32 * tx;
                data.push((top * (1.0 - ty) + bottom * ty).round() as u8);
            }
        }
    }
    Image {
        width,
        height,
        format: image.format,
        data,
    }
}

fn map_values(mut image: Image, f: impl Fn(u8) -> u8) -> Image {
    for v in &mut image.data {
        *v = f(*v);
    }
    image
}

fn grayscale(image: Image) -> Image {
    if image.format == ImageFormat::Grey {
        return image;
    }
    let data = image
        .data
        .chunks_exact(3)
        .map(|p| ((77 * p[0] as u32 + 150 * p[1] as u32 + 29 * p[2] as u32 + 128) >> 8) as u8)
        .collect();
    Image {
        width: image.width,
        height: image.height,
        format: ImageFormat::Grey,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3x2 greyscale image with values 0..6, row by row:
    /// ```text
    /// 0 1 2
    /// 3 4 5
    /// ```
    fn grid() -> Image {
        Image::new(3, 2, ImageFormat::Grey, (0..6).collect()).unwrap()
    }

    fn apply(t: Transform, image: Image) -> Image {
        t.apply(image).unwrap()
    }

    #[test]
    fn rotate_90_is_clockwise() {
        let out = apply(Transform::Rotate(Rotation::Deg90), grid());
        assert_eq!((out.width, out.height), (2, 3));
        assert_eq!(out.data, vec![3, 0, 4, 1, 5, 2]);
    }

    #[test]
    fn rotate_180_and_270() {
        assert_eq!(
            apply(Transform::Rotate(Rotation::Deg180), grid()).data,
            vec![5, 4, 3, 2, 1, 0]
        );
        let out = apply(Transform::Rotate(Rotation::Deg270), grid());
        assert_eq!((out.width, out.height), (2, 3));
        assert_eq!(out.data, vec![2, 5, 1, 4, 0, 3]);
    }

    #[test]
    fn four_quarter_turns_are_the_identity() {
        let mut image = grid();
        for _ in 0..4 {
            image = apply(Transform::Rotate(Rotation::Deg90), image);
        }
        assert_eq!(image, grid());
    }

    #[test]
    fn flips() {
        assert_eq!(
            apply(Transform::Flip(FlipAxis::Horizontal), grid()).data,
            vec![2, 1, 0, 5, 4, 3]
        );
        assert_eq!(
            apply(Transform::Flip(FlipAxis::Vertical), grid()).data,
            vec![3, 4, 5, 0, 1, 2]
        );
    }

    #[test]
    fn crop_takes_the_window_and_rejects_out_of_bounds() {
        let out = apply(
            Transform::Crop {
                x: 1,
                y: 0,
                width: 2,
                height: 2,
            },
            grid(),
        );
        assert_eq!((out.width, out.height, out.data), (2, 2, vec![1, 2, 4, 5]));
        assert!(
            Transform::Crop {
                x: 2,
                y: 0,
                width: 2,
                height: 1
            }
            .apply(grid())
            .is_err()
        );
    }

    #[test]
    fn resize_halves_uniform_blocks_exactly() {
        // 4x2 image of two uniform 2x2 blocks: 10 on the left, 50 on the right.
        let image = Image::new(
            4,
            2,
            ImageFormat::Grey,
            vec![10, 10, 50, 50, 10, 10, 50, 50],
        )
        .unwrap();
        let out = apply(
            Transform::Resize {
                width: 2,
                height: 1,
            },
            image,
        );
        assert_eq!(out.data, vec![10, 50]);
    }

    #[test]
    fn resize_upscale_interpolates_between_pixels() {
        let image = Image::new(2, 1, ImageFormat::Grey, vec![0, 100]).unwrap();
        let out = apply(
            Transform::Resize {
                width: 4,
                height: 1,
            },
            image,
        );
        assert_eq!(out.data, vec![0, 25, 75, 100]);
    }

    #[test]
    fn brightness_saturates() {
        let image = Image::new(3, 1, ImageFormat::Grey, vec![0, 100, 250]).unwrap();
        assert_eq!(
            apply(Transform::Brightness(10), image.clone()).data,
            vec![10, 110, 255]
        );
        assert_eq!(
            apply(Transform::Brightness(-20), image).data,
            vec![0, 80, 230]
        );
    }

    #[test]
    fn contrast_scales_around_mid_grey() {
        let image = Image::new(3, 1, ImageFormat::Grey, vec![28, 128, 228]).unwrap();
        assert_eq!(
            apply(Transform::Contrast(2.0), image.clone()).data,
            vec![0, 128, 255]
        );
        assert_eq!(
            apply(Transform::Contrast(0.5), image).data,
            vec![78, 128, 178]
        );
    }

    #[test]
    fn gamma_keeps_the_ends_and_bends_the_middle() {
        let image = Image::new(3, 1, ImageFormat::Grey, vec![0, 64, 255]).unwrap();
        assert_eq!(
            apply(Transform::Gamma(0.5), image.clone()).data,
            vec![0, 128, 255]
        );
        assert_eq!(apply(Transform::Gamma(1.0), image).data, vec![0, 64, 255]);
    }

    #[test]
    fn grayscale_uses_bt601_weights() {
        let image = Image::new(
            3,
            1,
            ImageFormat::Rgb,
            vec![255, 0, 0, 0, 255, 0, 255, 255, 255],
        )
        .unwrap();
        let out = apply(Transform::Grayscale, image);
        assert_eq!(out.format, ImageFormat::Grey);
        assert_eq!(out.data, vec![77, 149, 255]);
    }

    #[test]
    fn transforms_keep_rgb_pixels_together() {
        let image = Image::new(2, 1, ImageFormat::Rgb, vec![1, 2, 3, 4, 5, 6]).unwrap();
        assert_eq!(
            apply(Transform::Flip(FlipAxis::Horizontal), image).data,
            vec![4, 5, 6, 1, 2, 3]
        );
    }

    #[test]
    fn pipeline_applies_steps_in_order() {
        let pipeline = Pipeline {
            steps: vec![
                Transform::Crop {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 1,
                },
                Transform::Brightness(10),
            ],
        };
        assert_eq!(pipeline.apply(grid()).unwrap().data, vec![10, 11]);
    }
}
