use image::GenericImageView;

pub const DEFAULT_THRESHOLD: u8 = 20;
pub const DEFAULT_MAX_BRIGHT_RATIO: f64 = 0.01;
pub const EDGE_INSET: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MarginSensitivity {
    Strict,
    #[default]
    Normal,
    Tolerant,
}

impl MarginSensitivity {
    pub fn threshold(self) -> u8 {
        match self {
            MarginSensitivity::Strict => 10,
            MarginSensitivity::Normal => DEFAULT_THRESHOLD,
            MarginSensitivity::Tolerant => 40,
        }
    }

    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            Some("strict") => MarginSensitivity::Strict,
            Some("tolerant") => MarginSensitivity::Tolerant,
            _ => MarginSensitivity::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl BarSide {
    pub fn as_str(self) -> &'static str {
        match self {
            BarSide::Left => "left",
            BarSide::Right => "right",
            BarSide::Top => "top",
            BarSide::Bottom => "bottom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BarMeasurement {
    pub side: BarSide,
    pub passed: bool,
    pub bright_percent: f64,
    pub max_luminance: u8,
    pub threshold: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

pub fn target_box(width: u32, height: u32, target_ratio: f64) -> Rect {
    if width == 0 || height == 0 || target_ratio <= 0.0 {
        return Rect::new(0, 0, width, height);
    }

    let current_ratio = (width as f64) / (height as f64);
    if current_ratio > target_ratio {
        let target_width = (((height as f64) * target_ratio).round() as u32).min(width);
        let offset_x = (width - target_width) / 2;
        Rect::new(offset_x, 0, offset_x + target_width, height)
    } else {
        let target_height = (((width as f64) / target_ratio).round() as u32).min(height);
        let offset_y = (height - target_height) / 2;
        Rect::new(0, offset_y, width, offset_y + target_height)
    }
}

fn inset_region(img: &image::DynamicImage, r: Rect) -> Option<Rect> {
    let (img_w, img_h) = img.dimensions();
    let img_rect = Rect::new(0, 0, img_w, img_h);
    let inter = r.intersect(&img_rect)?;

    let (min_x, max_x) = if inter.width() > 2 * EDGE_INSET {
        (inter.min_x + EDGE_INSET, inter.max_x - EDGE_INSET)
    } else {
        (inter.min_x, inter.max_x)
    };

    let (min_y, max_y) = if inter.height() > 2 * EDGE_INSET {
        (inter.min_y + EDGE_INSET, inter.max_y - EDGE_INSET)
    } else {
        (inter.min_y, inter.max_y)
    };

    Some(Rect::new(min_x, min_y, max_x, max_y))
}

fn round_percent(bright_pixels: u64, total_pixels: u64) -> f64 {
    let raw_pct = (bright_pixels as f64 / total_pixels as f64) * 100.0;
    (raw_pct * 100.0).round() / 100.0
}

fn scan_region_luminance(
    img: &image::DynamicImage,
    region: Rect,
    threshold: u8,
    early_exit: bool,
) -> (bool, f64, u8) {
    let total_pixels = region.width() as u64 * region.height() as u64;
    if total_pixels == 0 {
        return (true, 0.0, 0);
    }

    let threshold_val = threshold as u32;
    let max_bright_pixels = ((total_pixels as f64) * DEFAULT_MAX_BRIGHT_RATIO).floor() as u64;
    let mut bright_pixels: u64 = 0;
    let mut max_luminance: u8 = 0;

    for y in region.min_y..region.max_y {
        for x in region.min_x..region.max_x {
            let pixel = img.get_pixel(x, y);
            let y_lum = (299 * pixel[0] as u32 + 587 * pixel[1] as u32 + 114 * pixel[2] as u32) / 1000;
            let y_u8 = y_lum as u8;
            if y_u8 > max_luminance {
                max_luminance = y_u8;
            }
            if y_lum > threshold_val {
                bright_pixels += 1;
                if early_exit && bright_pixels > max_bright_pixels {
                    return (false, round_percent(bright_pixels, total_pixels), max_luminance);
                }
            }
        }
    }

    (
        bright_pixels <= max_bright_pixels,
        round_percent(bright_pixels, total_pixels),
        max_luminance,
    )
}

pub fn measure_bar(
    img: &image::DynamicImage,
    side: BarSide,
    r: Rect,
    threshold: u8,
) -> Option<BarMeasurement> {
    let region = inset_region(img, r)?;
    let (passed, bright_percent, max_luminance) =
        scan_region_luminance(img, region, threshold, false);
    Some(BarMeasurement {
        side,
        passed,
        bright_percent,
        max_luminance,
        threshold,
    })
}

pub fn measure_bars(img: &image::DynamicImage, box_rect: Rect, threshold: u8) -> Vec<BarMeasurement> {
    let (width, height) = img.dimensions();
    let candidates = [
        (BarSide::Left, Rect::new(0, 0, box_rect.min_x, height)),
        (BarSide::Right, Rect::new(box_rect.max_x, 0, width, height)),
        (BarSide::Top, Rect::new(0, 0, width, box_rect.min_y)),
        (BarSide::Bottom, Rect::new(0, box_rect.max_y, width, height)),
    ];

    candidates
        .into_iter()
        .filter_map(|(side, rect)| measure_bar(img, side, rect, threshold))
        .collect()
}

pub fn format_failed_bars(measurements: &[BarMeasurement]) -> String {
    measurements
        .iter()
        .filter(|m| !m.passed)
        .map(|m| {
            format!(
                "{} bar {:.2}% bright, max lum {} > {}",
                m.side.as_str(),
                m.bright_percent,
                m.max_luminance,
                m.threshold
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn is_black_region(img: &image::DynamicImage, r: Rect, threshold: u8) -> bool {
    let Some(region) = inset_region(img, r) else {
        return true;
    };
    let (passed, _, _) = scan_region_luminance(img, region, threshold, true);
    passed
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn test_target_box() {
        let b1 = target_box(2560, 1080, 16.0 / 9.0);
        assert_eq!(b1, Rect::new(320, 0, 2240, 1080));

        let b2 = target_box(1920, 1200, 16.0 / 9.0);
        assert_eq!(b2, Rect::new(0, 60, 1920, 1140));

        let b3 = target_box(1920, 1080, 16.0 / 9.0);
        assert_eq!(b3, Rect::new(0, 0, 1920, 1080));
    }

    #[test]
    fn test_is_black_region() {
        let img = RgbaImage::from_pixel(100, 100, Rgba([10, 10, 10, 255]));
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        assert!(is_black_region(&dyn_img, Rect::new(0, 0, 50, 50), 20));

        let bright_img = RgbaImage::from_pixel(100, 100, Rgba([200, 200, 200, 255]));
        let dyn_bright = image::DynamicImage::ImageRgba8(bright_img);
        assert!(!is_black_region(&dyn_bright, Rect::new(0, 0, 50, 50), 20));
    }

    #[test]
    fn test_is_black_region_ignores_single_bright_edge_column() {
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 255]));
        for y in 0..100 {
            img.put_pixel(0, y, Rgba([231, 231, 231, 255]));
            img.put_pixel(49, y, Rgba([231, 231, 231, 255]));
        }
        let dyn_img = image::DynamicImage::ImageRgba8(img);

        assert!(is_black_region(&dyn_img, Rect::new(0, 0, 50, 100), DEFAULT_THRESHOLD));
    }

    #[test]
    fn test_is_black_region_allows_scattered_noise_below_tolerance() {
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 255]));
        for x in 10..30 {
            img.put_pixel(x, 50, Rgba([200, 200, 200, 255]));
        }
        let dyn_img = image::DynamicImage::ImageRgba8(img);

        assert!(is_black_region(&dyn_img, Rect::new(0, 0, 50, 100), DEFAULT_THRESHOLD));
    }

    #[test]
    fn test_is_black_region_rejects_margin_exceeding_tolerance() {
        let mut img = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 255]));
        for y in 10..20 {
            for x in 10..30 {
                img.put_pixel(x, y, Rgba([200, 200, 200, 255]));
            }
        }
        let dyn_img = image::DynamicImage::ImageRgba8(img);

        assert!(!is_black_region(&dyn_img, Rect::new(0, 0, 50, 100), DEFAULT_THRESHOLD));
    }
}
