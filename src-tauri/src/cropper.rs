use std::fs;
use std::path::Path;
use image::{GenericImageView, ImageEncoder};
use crate::metadata::{preserve_image_metadata, FileTimestamps};

pub const DEFAULT_THRESHOLD: u8 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Directory,
    Replace,
}

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

fn encode_and_save(
    img: &image::DynamicImage,
    src_path: &Path,
    target_path: &Path,
    ext: &str,
    quality: u8,
) -> Result<(), String> {
    let mut encoded_buf = Vec::new();

    if ext == "png" {
        let encoder = image::codecs::png::PngEncoder::new(&mut encoded_buf);
        encoder
            .write_image(
                img.to_rgba8().as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| format!("png encode error: {e}"))?;
    } else {
        let q = if quality == 0 || quality > 100 { 95 } else { quality };
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded_buf, q);
        encoder
            .write_image(
                img.to_rgb8().as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| format!("jpeg encode error: {e}"))?;
    }

    let final_bytes = match fs::read(src_path) {
        Ok(src_bytes) => preserve_image_metadata(&src_bytes, &encoded_buf, ext),
        Err(_) => encoded_buf,
    };

    fs::write(target_path, final_bytes).map_err(|e| format!("failed to write output file: {e}"))?;
    Ok(())
}

fn handle_unmodified(
    src_path: &Path,
    dest_path: Option<&std::path::PathBuf>,
    mode: OutputMode,
    reason: &str,
    status_on_copy: Status,
) -> CropResult {
    match mode {
        OutputMode::Directory => {
            if let Some(target) = dest_path {
                let timestamps = FileTimestamps::from_path(src_path);
                if let Err(e) = fs::copy(src_path, target) {
                    return CropResult {
                        path: src_path.to_string_lossy().to_string(),
                        status: Status::Error,
                        message: format!("copy error: {e}"),
                    };
                }
                if let Some(ts) = timestamps {
                    let _ = ts.apply_to(target);
                }
            }
            CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: status_on_copy,
                message: format!("{reason}; copied"),
            }
        }
        OutputMode::Replace => CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Skipped,
            message: format!("{reason}; kept unmodified"),
        },
    }
}

fn save_cropped(
    cropped: &image::DynamicImage,
    src_path: &Path,
    dest_path: Option<&std::path::PathBuf>,
    filename: &str,
    ext: &str,
    quality: u8,
    mode: OutputMode,
) -> Result<(), String> {
    let timestamps = FileTimestamps::from_path(src_path);

    match mode {
        OutputMode::Directory => {
            let target = dest_path.ok_or_else(|| "missing destination directory".to_string())?;
            encode_and_save(cropped, src_path, target, ext, quality)?;
            if let Some(ts) = timestamps {
                let _ = ts.apply_to(target);
            }
            Ok(())
        }
        OutputMode::Replace => {
            let parent = src_path.parent().unwrap_or_else(|| Path::new("."));
            let temp_path = parent.join(format!(".{}_{}.tmp_crop", std::process::id(), filename));

            if let Err(e) = encode_and_save(cropped, src_path, &temp_path, ext, quality) {
                let _ = fs::remove_file(&temp_path);
                return Err(e);
            }

            let res = fs::rename(&temp_path, src_path).or_else(|rename_err| {
                fs::copy(&temp_path, src_path)
                    .map(|_| ())
                    .map_err(|copy_err| format!("{copy_err} (rename error: {rename_err})"))
            });

            let _ = fs::remove_file(&temp_path);
            res.map_err(|e| format!("failed to replace original file: {e}"))?;

            if let Some(ts) = timestamps {
                let _ = ts.apply_to(src_path);
            }
            Ok(())
        }
    }
}

pub fn process(
    src_path: &Path,
    out_dir: &Path,
    target_ratio: f64,
    threshold: u8,
    quality: u8,
    mode: OutputMode,
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

    let dest_path = if mode == OutputMode::Directory {
        if let Err(e) = fs::create_dir_all(out_dir) {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Error,
                message: format!("failed to create output dir: {e}"),
            };
        }

        let p = out_dir.join(&filename);
        if p.exists() {
            return CropResult {
                path: src_path.to_string_lossy().to_string(),
                status: Status::Skipped,
                message: "already exists in output".to_string(),
            };
        }
        Some(p)
    } else {
        None
    };

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
        return handle_unmodified(
            src_path,
            dest_path.as_ref(),
            mode,
            "already matches target ratio",
            Status::Copied,
        );
    }

    // Verify candidate margin regions are black
    let left_bar = Rect::new(0, 0, box_rect.min_x, height);
    let right_bar = Rect::new(box_rect.max_x, 0, width, height);
    let top_bar = Rect::new(0, 0, width, box_rect.min_y);
    let bottom_bar = Rect::new(0, box_rect.max_y, width, height);

    let bars_are_black = is_black_region(&img, left_bar, threshold)
        && is_black_region(&img, right_bar, threshold)
        && is_black_region(&img, top_bar, threshold)
        && is_black_region(&img, bottom_bar, threshold);

    if !bars_are_black {
        return handle_unmodified(
            src_path,
            dest_path.as_ref(),
            mode,
            "non-black margins",
            Status::Copied,
        );
    }

    let cropped = img.crop_imm(
        box_rect.min_x,
        box_rect.min_y,
        box_rect.width(),
        box_rect.height(),
    );

    let ext = src_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if let Err(e) = save_cropped(
        &cropped,
        src_path,
        dest_path.as_ref(),
        &filename,
        &ext,
        quality,
        mode,
    ) {
        return CropResult {
            path: src_path.to_string_lossy().to_string(),
            status: Status::Error,
            message: e,
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
        let res = process(&src_path, &out_dir, 1.5, 20, 95, OutputMode::Directory);
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

        let res = process(&src_path, &out_dir, 1.5, 20, 95, OutputMode::Directory);
        assert_eq!(res.status, Status::Copied);

        let out_file = out_dir.join("test_bright.png");
        assert!(out_file.exists());
        let loaded = image::open(&out_file).unwrap();
        assert_eq!(loaded.width(), 200);
        assert_eq!(loaded.height(), 100);
    }

    #[test]
    fn test_process_replace_mode_cropped() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_replace_pillarbox.png");
        let dummy_out_dir = dir.path().join("unused_output");

        // Create 200x100 image with 25px black bars on left/right and bright center
        let mut img = RgbaImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                if x < 25 || x >= 175 {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                } else {
                    img.put_pixel(x, y, Rgba([200, 200, 200, 255]));
                }
            }
        }
        img.save(&src_path).unwrap();

        let res = process(&src_path, &dummy_out_dir, 1.5, 20, 95, OutputMode::Replace);
        assert_eq!(res.status, Status::Cropped);

        // The original file must have been replaced in-place
        assert!(src_path.exists());
        let loaded = image::open(&src_path).unwrap();
        assert_eq!(loaded.width(), 150);
        assert_eq!(loaded.height(), 100);
        // The dummy output dir should not contain files
        assert!(!dummy_out_dir.exists());
    }

    #[test]
    fn test_process_replace_mode_exact_ratio_unmodified() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_exact.png");
        let dummy_out_dir = dir.path().join("unused_output");

        let mut img = RgbaImage::new(150, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([100, 100, 100, 255]);
        }
        img.save(&src_path).unwrap();

        let res = process(&src_path, &dummy_out_dir, 1.5, 20, 95, OutputMode::Replace);
        assert_eq!(res.status, Status::Skipped);

        let loaded = image::open(&src_path).unwrap();
        assert_eq!(loaded.width(), 150);
        assert_eq!(loaded.height(), 100);
    }

    #[test]
    fn test_timestamps_restored_on_replace_mode() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_replace_time.png");
        let dummy_out_dir = dir.path().join("unused_output");

        let mut img = RgbaImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                if x < 25 || x >= 175 {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                } else {
                    img.put_pixel(x, y, Rgba([200, 200, 200, 255]));
                }
            }
        }
        img.save(&src_path).unwrap();

        let past_time = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let mut times = std::fs::FileTimes::new().set_modified(past_time);
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTimesExt;
            times = times.set_created(past_time);
        }
        let file = std::fs::OpenOptions::new().write(true).open(&src_path).unwrap();
        file.set_times(times).unwrap();
        drop(file);

        let res = process(&src_path, &dummy_out_dir, 1.5, 20, 95, OutputMode::Replace);
        assert_eq!(res.status, Status::Cropped);

        let new_meta = std::fs::metadata(&src_path).unwrap();
        let new_mod_time = new_meta.modified().unwrap();
        let diff = if new_mod_time > past_time {
            new_mod_time.duration_since(past_time).unwrap()
        } else {
            past_time.duration_since(new_mod_time).unwrap()
        };
        assert!(diff.as_secs() <= 2, "Modified time should be preserved across Replace mode");
    }

    #[test]
    fn test_process_directory_mode_preserves_metadata_and_timestamps() {
        use img_parts::jpeg::{Jpeg, JpegSegment};
        use image::{Rgb, RgbImage};

        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_src.jpg");
        let out_dir = dir.path().join("output");

        // 200x100 image with 25px black margins on left/right
        let mut img = RgbImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                if x < 25 || x >= 175 {
                    img.put_pixel(x, y, Rgb([0, 0, 0]));
                } else {
                    img.put_pixel(x, y, Rgb([180, 180, 180]));
                }
            }
        }

        let mut raw_jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut raw_jpeg)
            .write_image(
                img.as_raw(),
                200,
                100,
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();

        let mut src_jpeg = Jpeg::from_bytes(bytes::Bytes::copy_from_slice(&raw_jpeg)).unwrap();
        let custom_exif = JpegSegment::new_with_contents(
            0xE1,
            bytes::Bytes::from_static(b"Exif\0\0special_steam_tag"),
        );
        src_jpeg.segments_mut().insert(1, custom_exif);
        let src_bytes = src_jpeg.encoder().bytes().to_vec();
        std::fs::write(&src_path, src_bytes).unwrap();

        let past_time = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        let mut times = std::fs::FileTimes::new().set_modified(past_time);
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileTimesExt;
            times = times.set_created(past_time);
        }
        let file = std::fs::OpenOptions::new().write(true).open(&src_path).unwrap();
        file.set_times(times).unwrap();
        drop(file);

        let res = process(&src_path, &out_dir, 1.5, 20, 95, OutputMode::Directory);
        assert_eq!(res.status, Status::Cropped);

        let out_path = out_dir.join("test_src.jpg");
        assert!(out_path.exists());

        // Verify metadata was transferred
        let out_bytes = std::fs::read(&out_path).unwrap();
        let out_jpeg = Jpeg::from_bytes(bytes::Bytes::copy_from_slice(&out_bytes)).unwrap();
        let has_exif = out_jpeg.segments().iter().any(|s| {
            s.marker() == 0xE1 && s.contents().starts_with(b"Exif\0\0special_steam_tag")
        });
        assert!(has_exif, "Metadata should be transferred to cropped output file");

        // Verify timestamp was preserved
        let out_meta = std::fs::metadata(&out_path).unwrap();
        let out_mod = out_meta.modified().unwrap();
        let diff = if out_mod > past_time {
            out_mod.duration_since(past_time).unwrap()
        } else {
            past_time.duration_since(out_mod).unwrap()
        };
        assert!(diff.as_secs() <= 2, "Modified timestamp should be copied to output file");
    }
}
