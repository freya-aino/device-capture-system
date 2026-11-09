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
        if let Some(lut) = self.value_lut() {
            return Ok(apply_lut(image, &lut));
        }
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
            Transform::Grayscale => Ok(grayscale(image)),
            Transform::Brightness(_) | Transform::Contrast(_) | Transform::Gamma(_) => {
                unreachable!("value transforms are applied through their lookup table")
            }
        }
    }

    /// The lookup table of a transform that maps every channel value independently (brightness,
    /// contrast, gamma), or `None` for transforms that move pixels.
    fn value_lut(&self) -> Option<[u8; 256]> {
        let f: &dyn Fn(f32) -> f32 = match self {
            Transform::Brightness(offset) => &move |v| v + *offset as f32,
            Transform::Contrast(factor) => &move |v| (v - 128.0) * factor + 128.0,
            Transform::Gamma(gamma) => &move |v| 255.0 * (v / 255.0).powf(*gamma),
            _ => return None,
        };
        let mut lut = [0u8; 256];
        for (i, out) in lut.iter_mut().enumerate() {
            *out = f(i as f32).round().clamp(0.0, 255.0) as u8;
        }
        Some(lut)
    }
}

/// An ordered list of transforms for one device.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pipeline {
    pub steps: Vec<Transform>,
}

impl Pipeline {
    /// Runs the steps in order. Consecutive value transforms (brightness, contrast, gamma) are
    /// composed into one lookup table and applied in a single pass.
    pub fn apply(&self, mut image: Image) -> Result<Image> {
        let mut pending: Option<[u8; 256]> = None;
        for step in &self.steps {
            match step.value_lut() {
                Some(lut) => {
                    pending = Some(match pending {
                        Some(before) => before.map(|v| lut[v as usize]),
                        None => lut,
                    });
                }
                None => {
                    if let Some(lut) = pending.take() {
                        image = apply_lut(image, &lut);
                    }
                    image = step.apply(image)?;
                }
            }
        }
        if let Some(lut) = pending {
            image = apply_lut(image, &lut);
        }
        Ok(image)
    }
}

/// Pixel-moving transforms are generic over the channel count, so each pixel is a fixed-size copy
/// rather than a `memcpy` call.
fn with_channels(
    image: &Image,
    f3: impl FnOnce() -> Vec<u8>,
    f1: impl FnOnce() -> Vec<u8>,
) -> Vec<u8> {
    match image.format.channels() {
        3 => f3(),
        _ => f1(),
    }
}

fn pixels<const C: usize>(data: &[u8]) -> &[[u8; C]] {
    let (pixels, rest) = data.as_chunks::<C>();
    debug_assert!(rest.is_empty());
    pixels
}

fn pixels_mut<const C: usize>(data: &mut [u8]) -> &mut [[u8; C]] {
    let (pixels, rest) = data.as_chunks_mut::<C>();
    debug_assert!(rest.is_empty());
    pixels
}

/// Quarter turn. Walks the image in square tiles so reads and writes both stay in cache.
fn quarter_turn<const C: usize>(data: &[u8], w: usize, h: usize, clockwise: bool) -> Vec<u8> {
    const TILE: usize = 64;
    let src = pixels::<C>(data);
    let mut out = vec![0u8; data.len()];
    let dst = pixels_mut::<C>(&mut out);
    // The output is h wide and w tall.
    for ty in (0..h).step_by(TILE) {
        for tx in (0..w).step_by(TILE) {
            for y in ty..(ty + TILE).min(h) {
                for x in tx..(tx + TILE).min(w) {
                    let (ox, oy) = if clockwise {
                        (h - 1 - y, x)
                    } else {
                        (y, w - 1 - x)
                    };
                    dst[oy * h + ox] = src[y * w + x];
                }
            }
        }
    }
    out
}

fn reverse_pixels<const C: usize>(data: &[u8], row_len: usize) -> Vec<u8> {
    let mut out = vec![0u8; data.len()];
    for (dst_row, src_row) in out
        .chunks_exact_mut(row_len * C)
        .zip(data.chunks_exact(row_len * C))
    {
        for (d, s) in pixels_mut::<C>(dst_row)
            .iter_mut()
            .zip(pixels::<C>(src_row).iter().rev())
        {
            *d = *s;
        }
    }
    out
}

fn rotate(image: &Image, rotation: Rotation) -> Image {
    let (w, h) = (image.width as usize, image.height as usize);
    let (width, height, data) = match rotation {
        Rotation::Deg90 | Rotation::Deg270 => {
            let cw = rotation == Rotation::Deg90;
            let data = with_channels(
                image,
                || quarter_turn::<3>(&image.data, w, h, cw),
                || quarter_turn::<1>(&image.data, w, h, cw),
            );
            (image.height, image.width, data)
        }
        // Reversing the whole pixel sequence turns the image upside down and mirrors it.
        Rotation::Deg180 => {
            let n = w * h;
            let data = with_channels(
                image,
                || reverse_pixels::<3>(&image.data, n),
                || reverse_pixels::<1>(&image.data, n),
            );
            (image.width, image.height, data)
        }
    };
    Image {
        width,
        height,
        format: image.format,
        data,
    }
}

fn flip(image: &Image, axis: FlipAxis) -> Image {
    let stride = image.width as usize * image.format.channels();
    let data = match axis {
        FlipAxis::Horizontal => {
            let w = image.width as usize;
            with_channels(
                image,
                || reverse_pixels::<3>(&image.data, w),
                || reverse_pixels::<1>(&image.data, w),
            )
        }
        FlipAxis::Vertical => {
            let mut data = Vec::with_capacity(image.data.len());
            for row in image.data.chunks_exact(stride).rev() {
                data.extend_from_slice(row);
            }
            data
        }
    };
    Image {
        width: image.width,
        height: image.height,
        format: image.format,
        data,
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
    let c = image.format.channels();
    let stride = image.width as usize * c;
    let (left, len) = (x as usize * c, width as usize * c);
    let mut data = Vec::with_capacity(len * height as usize);
    for row in image
        .data
        .chunks_exact(stride)
        .skip(y as usize)
        .take(height as usize)
    {
        data.extend_from_slice(&row[left..left + len]);
    }
    Ok(Image {
        width,
        height,
        format: image.format,
        data,
    })
}

/// Bilinear resize with pixel centres aligned (the OpenCV `INTER_LINEAR` convention).
fn resize(image: &Image, width: u32, height: u32) -> Image {
    let c = image.format.channels();
    let stride = image.width as usize * c;
    // For each output index along one axis: the two neighbouring source indices and the weight of
    // the second.
    let taps = |out: u32, src: u32| -> Vec<(usize, usize, f32)> {
        let scale = src as f32 / out as f32;
        (0..out)
            .map(|i| {
                let f = ((i as f32 + 0.5) * scale - 0.5).clamp(0.0, (src - 1) as f32);
                let i0 = f.floor() as usize;
                (i0, (i0 + 1).min(src as usize - 1), f.fract())
            })
            .collect()
    };
    let xs = taps(width, image.width);
    let ys = taps(height, image.height);
    let mut data = Vec::with_capacity(width as usize * height as usize * c);
    for &(y0, y1, ty) in &ys {
        let top = &image.data[y0 * stride..(y0 + 1) * stride];
        let bottom = &image.data[y1 * stride..(y1 + 1) * stride];
        for &(x0, x1, tx) in &xs {
            for ch in 0..c {
                let (a, b) = (top[x0 * c + ch] as f32, top[x1 * c + ch] as f32);
                let (d, e) = (bottom[x0 * c + ch] as f32, bottom[x1 * c + ch] as f32);
                let upper = a + (b - a) * tx;
                let lower = d + (e - d) * tx;
                data.push((upper + (lower - upper) * ty).round() as u8);
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

fn apply_lut(mut image: Image, lut: &[u8; 256]) -> Image {
    for v in &mut image.data {
        *v = lut[*v as usize];
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
    fn fused_value_steps_match_applying_them_one_by_one() {
        let image = Image::new(256, 1, ImageFormat::Grey, (0..=255).collect()).unwrap();
        let steps = vec![
            Transform::Brightness(-30),
            Transform::Contrast(1.7),
            Transform::Gamma(0.6),
        ];
        let mut one_by_one = image.clone();
        for step in &steps {
            one_by_one = step.apply(one_by_one).unwrap();
        }
        assert_eq!(Pipeline { steps }.apply(image).unwrap(), one_by_one);
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
