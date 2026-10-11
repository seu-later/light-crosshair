use std::{fs, path::Path};

use image::ImageError;

use crate::config::{MAX_SCALE_PERCENT, MIN_SCALE_PERCENT};

const MAX_DIMENSION: u32 = 4096;

#[derive(Clone, Debug)]
pub struct OriginalImage {
    pub width: u32,
    pub height: u32,
    rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ProcessedImage {
    pub width: u32,
    pub height: u32,
    pub premultiplied_bgra: Vec<u8>,
}

impl OriginalImage {
    pub fn load_png(path: &Path) -> Result<Self, ImageError> {
        let bytes = fs::read(path).map_err(ImageError::IoError)?;
        let decoded =
            image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
        let (width, height) = decoded.dimensions();
        if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(ImageError::Limits(image::error::LimitError::from_kind(
                image::error::LimitErrorKind::DimensionError,
            )));
        }
        Ok(Self {
            width,
            height,
            rgba: decoded.into_raw(),
        })
    }

    pub fn process(
        &self,
        tint: Option<[u8; 3]>,
        tint_exact: bool,
        scale_percent: u32,
    ) -> ProcessedImage {
        let scale_percent = scale_percent.clamp(MIN_SCALE_PERCENT, MAX_SCALE_PERCENT);
        let width = scaled_dimension(self.width, scale_percent);
        let height = scaled_dimension(self.height, scale_percent);
        let mut output = vec![0; width as usize * height as usize * 4];

        for y in 0..height {
            let source_y = y * self.height / height;
            for x in 0..width {
                let source_x = x * self.width / width;
                let source = ((source_y * self.width + source_x) * 4) as usize;
                let destination = ((y * width + x) * 4) as usize;
                let alpha = self.rgba[source + 3] as u16;
                let mut red = self.rgba[source] as u16;
                let mut green = self.rgba[source + 1] as u16;
                let mut blue = self.rgba[source + 2] as u16;

                if let Some(tint) = tint {
                    if tint_exact {
                        red = tint[0] as u16;
                        green = tint[1] as u16;
                        blue = tint[2] as u16;
                    } else {
                        red = (red * tint[0] as u16 + 127) / 255;
                        green = (green * tint[1] as u16 + 127) / 255;
                        blue = (blue * tint[2] as u16 + 127) / 255;
                    }
                }

                output[destination] = ((blue * alpha + 127) / 255) as u8;
                output[destination + 1] = ((green * alpha + 127) / 255) as u8;
                output[destination + 2] = ((red * alpha + 127) / 255) as u8;
                output[destination + 3] = alpha as u8;
            }
        }

        ProcessedImage {
            width,
            height,
            premultiplied_bgra: output,
        }
    }
}

pub fn scaled_dimension(original: u32, scale_percent: u32) -> u32 {
    if scale_percent == 0 {
        return 0;
    }
    ((original as u64 * scale_percent.clamp(MIN_SCALE_PERCENT, MAX_SCALE_PERCENT) as u64 + 50)
        / 100)
        .max(1) as u32
}

#[cfg(test)]
mod tests {
    use super::{OriginalImage, scaled_dimension};

    #[test]
    fn scale_formula_matches_specification() {
        assert_eq!(scaled_dimension(32, 0), 0);
        assert_eq!(scaled_dimension(32, 20), 6);
        assert_eq!(scaled_dimension(32, 100), 32);
        assert_eq!(scaled_dimension(32, 150), 48);
        assert_eq!(scaled_dimension(32, 500), 160);
        assert_eq!(scaled_dimension(1, 20), 1);
    }

    #[test]
    fn tint_preserves_alpha_and_premultiplies_for_windows() {
        let image = OriginalImage {
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 128],
        };
        let processed = image.process(Some([255, 0, 0]), false, 100);
        assert_eq!(processed.premultiplied_bgra, [0, 0, 128, 128]);
    }

    #[test]
    fn exact_tint_replaces_non_white_source() {
        let image = OriginalImage {
            width: 1,
            height: 1,
            rgba: vec![255, 0, 0, 255],
        };
        let multiplied = image.process(Some([0, 255, 0]), false, 100);
        assert_eq!(multiplied.premultiplied_bgra, [0, 0, 0, 255]);
        let replaced = image.process(Some([0, 255, 0]), true, 100);
        assert_eq!(replaced.premultiplied_bgra, [0, 255, 0, 255]);
    }

    #[test]
    fn nearest_neighbor_does_not_invent_colors() {
        let image = OriginalImage {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
        };
        let processed = image.process(None, false, 200);
        assert_eq!(processed.premultiplied_bgra[0..4], [0, 0, 255, 255]);
        assert_eq!(processed.premultiplied_bgra[4..8], [0, 0, 255, 255]);
        assert_eq!(processed.premultiplied_bgra[8..12], [255, 0, 0, 255]);
        assert_eq!(processed.premultiplied_bgra[12..16], [255, 0, 0, 255]);
    }
}
