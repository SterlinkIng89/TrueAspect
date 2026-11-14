use std::fs;
use std::path::Path;
use image::{GenericImageView, ImageEncoder};
use crate::metadata::{preserve_image_metadata, FileTimestamps};

pub use crate::margin::{
    format_failed_bars, is_black_region, measure_bar, measure_bars, target_box, BarMeasurement,
    BarSide, MarginSensitivity, Rect, DEFAULT_MAX_BRIGHT_RATIO, DEFAULT_THRESHOLD, EDGE_INSET,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropReason {
    AlreadyMatchesRatio,
    NonBlackMargins,
    OutputExists,
    DecodeFailed,
    WriteFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CropResult {
    pub path: String,
    pub status: Status,
    pub message: String,
    pub reason: Option<CropReason>,
    pub dimensions: Option<ImageDimensions>,
    pub target_box: Option<Rect>,
    pub margin_measurements: Option<Vec<BarMeasurement>>,
}

impl CropResult {
    fn simple(path: String, status: Status, reason: Option<CropReason>, message: String) -> Self {
        Self {
            path,
            status,
            message,
            reason,
            dimensions: None,
            target_box: None,
            margin_measurements: None,
        }
    }

    fn with_context(
        path: String,
        status: Status,
        reason: Option<CropReason>,
        message: String,
        dimensions: ImageDimensions,
        target_box: Rect,
        margin_measurements: Option<Vec<BarMeasurement>>,
    ) -> Self {
        Self {
            path,
            status,
            message,
            reason,
            dimensions: Some(dimensions),
            target_box: Some(target_box),
            margin_measurements,
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
    src_path_str: String,
    dest_path: Option<&std::path::PathBuf>,
    mode: OutputMode,
    reason: CropReason,
    reason_text: &str,
    status_on_copy: Status,
    dimensions: ImageDimensions,
    target_box: Rect,
    margin_measurements: Option<Vec<BarMeasurement>>,
) -> CropResult {
    match mode {
        OutputMode::Directory => {
            if let Some(target) = dest_path {
                let timestamps = FileTimestamps::from_path(src_path);
                if let Err(e) = fs::copy(src_path, target) {
                    return CropResult::with_context(
                        src_path_str,
                        Status::Error,
                        Some(CropReason::WriteFailed),
                        format!("copy error: {e}"),
                        dimensions,
                        target_box,
                        margin_measurements,
                    );
                }
                if let Some(ts) = timestamps {
                    let _ = ts.apply_to(target);
                }
            }
            CropResult::with_context(
                src_path_str,
                status_on_copy,
                Some(reason),
                format!("{reason_text}; copied"),
                dimensions,
                target_box,
                margin_measurements,
            )
        }
        OutputMode::Replace => CropResult::with_context(
            src_path_str,
            Status::Skipped,
            Some(reason),
            format!("{reason_text}; kept unmodified"),
            dimensions,
            target_box,
            margin_measurements,
        ),
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
    let src_path_str = src_path.to_string_lossy().to_string();
    let filename = match src_path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            return CropResult::simple(
                src_path_str,
                Status::Error,
                Some(CropReason::DecodeFailed),
                "invalid source path".to_string(),
            );
        }
    };

    let dest_path = if mode == OutputMode::Directory {
        if let Err(e) = fs::create_dir_all(out_dir) {
            return CropResult::simple(
                src_path_str,
                Status::Error,
                Some(CropReason::WriteFailed),
                format!("failed to create output dir: {e}"),
            );
        }

        let p = out_dir.join(&filename);
        if p.exists() {
            return CropResult::simple(
                src_path_str,
                Status::Skipped,
                Some(CropReason::OutputExists),
                "already exists in output".to_string(),
            );
        }
        Some(p)
    } else {
        None
    };

    let img = match image::open(src_path) {
        Ok(i) => i,
        Err(e) => {
            return CropResult::simple(
                src_path_str,
                Status::Error,
                Some(CropReason::DecodeFailed),
                format!("decode error: {e}"),
            );
        }
    };

    let (width, height) = img.dimensions();
    let dimensions = ImageDimensions { width, height };
    let box_rect = target_box(width, height, target_ratio);

    if box_rect.min_x == 0
        && box_rect.min_y == 0
        && box_rect.width() == width
        && box_rect.height() == height
    {
        return handle_unmodified(
            src_path,
            src_path_str,
            dest_path.as_ref(),
            mode,
            CropReason::AlreadyMatchesRatio,
            "already matches target ratio",
            Status::Copied,
            dimensions,
            box_rect,
            None,
        );
    }

    let measurements = measure_bars(&img, box_rect, threshold);
    if !measurements.iter().all(|m| m.passed) {
        let reason_text = format!("non-black margins ({})", format_failed_bars(&measurements));
        return handle_unmodified(
            src_path,
            src_path_str,
            dest_path.as_ref(),
            mode,
            CropReason::NonBlackMargins,
            &reason_text,
            Status::Copied,
            dimensions,
            box_rect,
            Some(measurements),
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
        return CropResult::with_context(
            src_path_str,
            Status::Error,
            Some(CropReason::WriteFailed),
            e,
            dimensions,
            box_rect,
            Some(measurements),
        );
    }

    CropResult::with_context(
        src_path_str,
        Status::Cropped,
        None,
        format!(
            "cropped {width}x{height} -> {}x{}",
            box_rect.width(),
            box_rect.height()
        ),
        dimensions,
        box_rect,
        Some(measurements),
    )
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
    fn test_process_crops_image_with_bright_edge_column_in_margin() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("test_edge_column.png");
        let out_dir = dir.path().join("unused_output");

        let mut img = RgbaImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                if x < 25 || x >= 175 {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                } else {
                    img.put_pixel(x, y, Rgba([180, 180, 180, 255]));
                }
            }
            img.put_pixel(0, y, Rgba([231, 231, 231, 255]));
            img.put_pixel(175, y, Rgba([231, 231, 231, 255]));
        }
        img.save(&src_path).unwrap();

        let res = process(&src_path, &out_dir, 1.5, DEFAULT_THRESHOLD, 95, OutputMode::Replace);
        assert_eq!(res.status, Status::Cropped);

        let loaded = image::open(&src_path).unwrap();
        assert_eq!(loaded.width(), 150);
        assert_eq!(loaded.height(), 100);
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

    #[test]
    fn test_process_structured_reasons_and_measurements() {
        let dir = tempdir().unwrap();
        let out_dir = dir.path().join("output");

        // 1. AlreadyMatchesRatio
        let exact_path = dir.path().join("exact.png");
        let exact_img = RgbaImage::from_pixel(150, 100, Rgba([100, 100, 100, 255]));
        exact_img.save(&exact_path).unwrap();
        let exact_res = process(&exact_path, &out_dir, 1.5, 20, 95, OutputMode::Replace);
        assert_eq!(exact_res.status, Status::Skipped);
        assert_eq!(exact_res.reason, Some(CropReason::AlreadyMatchesRatio));
        assert_eq!(
            exact_res.dimensions,
            Some(ImageDimensions {
                width: 150,
                height: 100
            })
        );
        assert_eq!(exact_res.target_box, Some(Rect::new(0, 0, 150, 100)));
        assert!(exact_res.margin_measurements.is_none());

        // 2. NonBlackMargins with measurements
        let bright_path = dir.path().join("bright_margins.png");
        let bright_img = RgbaImage::from_pixel(200, 100, Rgba([150, 150, 150, 255]));
        bright_img.save(&bright_path).unwrap();
        let bright_res = process(&bright_path, &out_dir, 1.5, 20, 95, OutputMode::Replace);
        assert_eq!(bright_res.status, Status::Skipped);
        assert_eq!(bright_res.reason, Some(CropReason::NonBlackMargins));
        assert_eq!(
            bright_res.dimensions,
            Some(ImageDimensions {
                width: 200,
                height: 100
            })
        );
        assert_eq!(bright_res.target_box, Some(Rect::new(25, 0, 175, 100)));
        let measurements = bright_res.margin_measurements.expect("expected measurements");
        assert_eq!(measurements.len(), 2);
        assert_eq!(measurements[0].side, BarSide::Left);
        assert!(!measurements[0].passed);
        assert_eq!(measurements[0].bright_percent, 100.0);
        assert_eq!(measurements[0].max_luminance, 150);
        assert_eq!(measurements[0].threshold, 20);
        assert_eq!(measurements[1].side, BarSide::Right);
        assert!(!measurements[1].passed);

        // 3. OutputExists in Directory mode
        let dup_path = dir.path().join("dup.png");
        let dup_img = RgbaImage::from_pixel(200, 100, Rgba([0, 0, 0, 255]));
        dup_img.save(&dup_path).unwrap();
        let first_dir_res = process(&dup_path, &out_dir, 1.5, 20, 95, OutputMode::Directory);
        assert_eq!(first_dir_res.status, Status::Cropped);
        assert!(first_dir_res.reason.is_none());
        let second_dir_res = process(&dup_path, &out_dir, 1.5, 20, 95, OutputMode::Directory);
        assert_eq!(second_dir_res.status, Status::Skipped);
        assert_eq!(second_dir_res.reason, Some(CropReason::OutputExists));

        // 4. DecodeFailed on corrupt/missing file
        let bad_path = dir.path().join("corrupt.png");
        std::fs::write(&bad_path, b"not an image").unwrap();
        let bad_res = process(&bad_path, &out_dir, 1.5, 20, 95, OutputMode::Replace);
        assert_eq!(bad_res.status, Status::Error);
        assert_eq!(bad_res.reason, Some(CropReason::DecodeFailed));
    }

    #[test]
    fn test_margin_sensitivity_thresholds() {
        assert_eq!(MarginSensitivity::Strict.threshold(), 10);
        assert_eq!(MarginSensitivity::Normal.threshold(), DEFAULT_THRESHOLD);
        assert_eq!(MarginSensitivity::Tolerant.threshold(), 40);
        assert_eq!(MarginSensitivity::from_str_opt(Some("strict")), MarginSensitivity::Strict);
        assert_eq!(MarginSensitivity::from_str_opt(Some("tolerant")), MarginSensitivity::Tolerant);
        assert_eq!(MarginSensitivity::from_str_opt(Some("unknown")), MarginSensitivity::Normal);
        assert_eq!(MarginSensitivity::from_str_opt(None), MarginSensitivity::Normal);

        let dir = tempdir().unwrap();
        let src_path = dir.path().join("near_black.png");
        let out_dir = dir.path().join("output");

        let mut img = RgbaImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200 {
                if x < 25 || x >= 175 {
                    img.put_pixel(x, y, Rgba([30, 30, 30, 255]));
                } else {
                    img.put_pixel(x, y, Rgba([180, 180, 180, 255]));
                }
            }
        }
        img.save(&src_path).unwrap();

        let normal_res = process(
            &src_path,
            &out_dir,
            1.5,
            MarginSensitivity::Normal.threshold(),
            95,
            OutputMode::Replace,
        );
        assert_eq!(normal_res.status, Status::Skipped);
        assert_eq!(normal_res.reason, Some(CropReason::NonBlackMargins));

        let tolerant_res = process(
            &src_path,
            &out_dir,
            1.5,
            MarginSensitivity::Tolerant.threshold(),
            95,
            OutputMode::Replace,
        );
        assert_eq!(tolerant_res.status, Status::Cropped);
    }
}
