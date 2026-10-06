use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;
use image::{GenericImageView, ImageEncoder};

pub const DEFAULT_THRESHOLD: u8 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Cropped,
    Copied,
    Skipped,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CropResult {
    pub path: String,
    pub status: Status,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl Rect {
    pub fn new(min_x: u32, min_y: u32, max_x: u32, max_y: u32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    pub fn width(&self) -> u32 {
        self.max_x.saturating_sub(self.min_x)
    }

    pub fn height(&self) -> u32 {
        self.max_y.saturating_sub(self.min_y)
    }

    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let min_x = self.min_x.max(other.min_x);
        let min_y = self.min_y.max(other.min_y);
        let max_x = self.max_x.min(other.max_x);
        let max_y = self.max_y.min(other.max_y);

        if min_x < max_x && min_y < max_y {
            Some(Rect::new(min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }
}

pub fn parse_ratio(s: &str) -> Result<f64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty ratio string".to_string());
    }

    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("invalid ratio format: {s}"));
        }

        let num: f64 = parts[0]
            .trim()
            .parse()
            .map_err(|e| format!("invalid numerator: {e}"))?;
        let den: f64 = parts[1]
            .trim()
            .parse()
            .map_err(|e| format!("invalid denominator: {e}"))?;

        if num <= 0.0 || den <= 0.0 {
            return Err("ratio values must be positive".to_string());
        }

        Ok(num / den)
    } else {
        let val: f64 = s.parse().map_err(|e| format!("invalid ratio float: {e}"))?;
        if val <= 0.0 {
            return Err("ratio must be positive".to_string());
        }
        Ok(val)
    }
}

pub fn target_box(width: u32, height: u32, target_ratio: f64) -> Rect {
    if width == 0 || height == 0 || target_ratio <= 0.0 {
        return Rect::new(0, 0, width, height);
    }

    let current_ratio = (width as f64) / (height as f64);
    if current_ratio > target_ratio {
        let mut target_width = ((height as f64) * target_ratio).round() as u32;
        if target_width > width {
            target_width = width;
        }
        let offset_x = (width - target_width) / 2;
        Rect::new(offset_x, 0, offset_x + target_width, height)
    } else {
        let mut target_height = ((width as f64) / target_ratio).round() as u32;
        if target_height > height {
            target_height = height;
        }
        let offset_y = (height - target_height) / 2;
        Rect::new(0, offset_y, width, offset_y + target_height)
    }
}

pub fn is_black_region(img: &image::DynamicImage, r: Rect, threshold: u8) -> bool {
    let (img_w, img_h) = img.dimensions();
    let img_rect = Rect::new(0, 0, img_w, img_h);

    let inter = match r.intersect(&img_rect) {
        Some(i) => i,
        None => return true,
    };

    let threshold_val = threshold as u32;

    for y in inter.min_y..inter.max_y {
        for x in inter.min_x..inter.max_x {
            let pixel = img.get_pixel(x, y);
            let r8 = pixel[0] as u32;
            let g8 = pixel[1] as u32;
            let b8 = pixel[2] as u32;
            let y_lum = (299 * r8 + 587 * g8 + 114 * b8) / 1000;
            if y_lum > threshold_val {
                return false;
            }
        }
    }

    true
}

pub fn process(
    src_path: &Path,
    out_dir: &Path,
    target_ratio: f64,
    threshold: u8,
    quality: u8,
) -> CropResult {
    let filename = match src_path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Error,
                message: "invalid source path".to_string(),
            }
        }
    };

    if let Err(e) = fs::create_dir_all(out_dir) {
        return CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Error,
            message: format!("failed to create output dir: {e}"),
        };
    }

    let dest_path = out_dir.join(&filename);
    if dest_path.exists() {
        return CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Skipped,
            message: "already exists in output".to_string(),
        };
    }

    let img = match image::open(src_path) {
        Ok(i) => i,
        Err(e) => {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Error,
                message: format!("decode error: {e}"),
            }
        }
    };

    let (width, height) = img.dimensions();
    let box_rect = target_box(width, height, target_ratio);

    if box_rect.min_x == 0
        && box_rect.min_y == 0
        && box_rect.width() == width
        && box_rect.height() == height
    {
        if let Err(e) = fs::copy(src_path, &dest_path) {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Error,
                message: format!("copy error: {e}"),
            };
        }
        return CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Copied,
            message: "already target ratio; copied".to_string(),
        };
    }

    // Verify the 4 candidate margin regions are black
    let left_bar = Rect::new(0, 0, box_rect.min_x, height);
    let right_bar = Rect::new(box_rect.max_x, 0, width, height);
    let top_bar = Rect::new(0, 0, width, box_rect.min_y);
    let bottom_bar = Rect::new(0, box_rect.max_y, width, height);

    let bars_are_black = is_black_region(&img, left_bar, threshold)
        && is_black_region(&img, right_bar, threshold)
        && is_black_region(&img, top_bar, threshold)
        && is_black_region(&img, bottom_bar, threshold);

    if !bars_are_black {
        if let Err(e) = fs::copy(src_path, &dest_path) {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Error,
                message: format!("copy error: {e}"),
            };
        }
        return CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Copied,
            message: "non-black margins; copied without cropping".to_string(),
        };
    }

    let cropped = img.crop_imm(
        box_rect.min_x,
        box_rect.min_y,
        box_rect.width(),
        box_rect.height(),
    );

    let out_file = match File::create(&dest_path) {
        Ok(f) => f,
        Err(e) => {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Error,
                message: format!("failed to create output file: {e}"),
            }
        }
    };
    let mut writer = BufWriter::new(out_file);

    let ext = src_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    let save_res = if ext == "png" {
        let encoder = image::codecs::png::PngEncoder::new(&mut writer);
        encoder.write_image(
            cropped.to_rgba8().as_raw(),
            cropped.width(),
            cropped.height(),
            image::ExtendedColorType::Rgba8,
        )
    } else {
        let q = if quality == 0 || quality > 100 { 95 } else { quality };
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, q);
        encoder.write_image(
            cropped.to_rgb8().as_raw(),
            cropped.width(),
            cropped.height(),
            image::ExtendedColorType::Rgb8,
        )
    };

    if let Err(e) = save_res {
        return CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Error,
            message: format!("encode error: {e}"),
        };
    }

    CropResult {
        path: src_path.to_string_lossy().to_string(),
        status: Status::Cropped,
        message: format!(
            "cropped {width}x{height} -> {}x{}",
            box_rect.width(),
            box_rect.height()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    #[test]
    fn test_parse_ratio_valid() {
        assert!((parse_ratio("16:9").unwrap() - 16.0 / 9.0).abs() < 1e-6);
        assert!((parse_ratio("4:3").unwrap() - 4.0 / 3.0).abs() < 1e-6);
        assert!((parse_ratio("21:9").unwrap() - 21.0 / 9.0).abs() < 1e-6);
        assert!((parse_ratio("1.7778").unwrap() - 1.7778).abs() < 1e-6);
        assert!((parse_ratio(" 16 : 9 ").unwrap() - 16.0 / 9.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_ratio_invalid() {
        assert!(parse_ratio("").is_err());
        assert!(parse_ratio("abc").is_err());
        assert!(parse_ratio("16:0").is_err());
        assert!(parse_ratio("-16:9").is_err());
        assert!(parse_ratio("16:9:1").is_err());
    }

    #[test]
    fn test_target_box() {
        // Pillarbox: 2560x1080 target 16:9 -> height 1080, width 1920, offset (2560-1920)/2 = 320
        let b1 = target_box(2560, 1080, 16.0 / 9.0);
        assert_eq!(b1, Rect::new(320, 0, 2240, 1080));

        // Letterbox: 1920x1200 target 16:9 -> width 1920, height 1080, offset (1200-1080)/2 = 60
        let b2 = target_box(1920, 1200, 16.0 / 9.0);
        assert_eq!(b2, Rect::new(0, 60, 1920, 1140));

        // Exact match
        let b3 = target_box(1920, 1080, 16.0 / 9.0);
        assert_eq!(b3, Rect::new(0, 0, 1920, 1080));
    }

    #[test]
    fn test_is_black_region() {
        let mut img = RgbaImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([10, 10, 10, 255]);
        }
        let dyn_img = image::DynamicImage::ImageRgba8(img);

        assert!(is_black_region(&dyn_img, Rect::new(0, 0, 50, 50), 20));

        let mut bright_img = RgbaImage::new(100, 100);
        for pixel in bright_img.pixels_mut() {
            *pixel = Rgba([200, 200, 200, 255]);
        }
        let dyn_bright = image::DynamicImage::ImageRgba8(bright_img);

        assert!(!is_black_region(&dyn_bright, Rect::new(0, 0, 50, 50), 20));
    }

    #[test]
    fn test_process_cropped() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_pillarbox.png");
        let out_dir = dir.path().join("output");

        // Create 200x100 image with 25px black bars on left/right and bright center
        let mut img = RgbaImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                if x < 25 || x >= 175 {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                } else {
                    img.put_pixel(x, y, Rgba([180, 180, 180, 255]));
                }
            }
        }
        img.save(&src_path).unwrap();

        // Target ratio 150:100 = 1.5
        let res = process(&src_path, &out_dir, 1.5, 20, 95);
        assert_eq!(res.status, Status::Cropped);

        let out_file = out_dir.join("test_pillarbox.png");
        assert!(out_file.exists());
        let loaded = image::open(&out_file).unwrap();
        assert_eq!(loaded.width(), 150);
        assert_eq!(loaded.height(), 100);
    }

    #[test]
    fn test_process_non_black_copied() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_bright.png");
        let out_dir = dir.path().join("output");

        let mut img = RgbaImage::new(200, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([150, 150, 150, 255]);
        }
        img.save(&src_path).unwrap();

        let res = process(&src_path, &out_dir, 1.5, 20, 95);
        assert_eq!(res.status, Status::Copied);

        let out_file = out_dir.join("test_bright.png");
        assert!(out_file.exists());
        let loaded = image::open(&out_file).unwrap();
        assert_eq!(loaded.width(), 200);
        assert_eq!(loaded.height(), 100);
    }
}
