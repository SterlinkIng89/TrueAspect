use image::imageops::FilterType;
use image::ImageEncoder;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug, thiserror::Error)]
pub enum ThumbError {
    #[error("file not allowed or forbidden: {0}")]
    Forbidden(String),
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("image decode failed: {0}")]
    DecodeError(String),
    #[error("image encode failed: {0}")]
    EncodeError(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct ThumbService {
    allowed: RwLock<HashSet<String>>,
    cache: Mutex<HashMap<String, Arc<Vec<u8>>>>,
    thumb_width: u32,
}

impl ThumbService {
    pub fn new(thumb_width: u32) -> Self {
        let width = if thumb_width == 0 { 420 } else { thumb_width };
        Self {
            allowed: RwLock::new(HashSet::new()),
            cache: Mutex::new(HashMap::new()),
            thumb_width: width,
        }
    }

    pub fn register_allowed<I, S>(&self, paths: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut allowed = self.allowed.write().unwrap();
        for p in paths {
            allowed.insert(p.as_ref().to_string());
        }
    }

    pub fn is_allowed(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        if self.allowed.read().unwrap().contains(path_str.as_ref()) {
            return true;
        }

        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            let lower = ext.to_lowercase();
            if (lower == "jpg" || lower == "jpeg" || lower == "png") && path.exists() {
                return true;
            }
        }

        false
    }

    pub fn render(&self, path_str: &str) -> Result<Arc<Vec<u8>>, ThumbError> {
        let path = PathBuf::from(path_str);
        if !self.is_allowed(&path) {
            return Err(ThumbError::Forbidden(path_str.to_string()));
        }

        let metadata = fs::metadata(&path).map_err(|_| ThumbError::NotFound(path_str.to_string()))?;
        let mod_time = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        hasher.update(mod_time.to_le_bytes());
        let cache_key = format!("{:x}", hasher.finalize());

        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(Arc::clone(cached));
            }
        }

        let img = image::open(&path).map_err(|e| ThumbError::DecodeError(e.to_string()))?;
        let (orig_w, orig_h) = (img.width(), img.height());

        let target_w = if orig_w < self.thumb_width { orig_w } else { self.thumb_width };
        let target_h = (((orig_h as f64) * (target_w as f64) / (orig_w as f64)).round() as u32).max(1);

        let resized = img.resize(target_w, target_h, FilterType::Triangle);

        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 80);
        encoder
            .write_image(
                resized.to_rgb8().as_raw(),
                resized.width(),
                resized.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| ThumbError::EncodeError(e.to_string()))?;

        let arc_data = Arc::new(buf);
        let mut cache = self.cache.lock().unwrap();
        cache.insert(cache_key, Arc::clone(&arc_data));

        Ok(arc_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    #[test]
    fn test_thumb_service_cache_and_render() {
        let service = ThumbService::new(420);
        let dir = tempdir().unwrap();
        let img_path = dir.path().join("thumb_test.png");

        let mut img = RgbaImage::new(1000, 500);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([100, 150, 200, 255]);
        }
        img.save(&img_path).unwrap();

        let path_str = img_path.to_string_lossy().to_string();
        service.register_allowed([&path_str]);

        // First render
        let bytes1 = service.render(&path_str).unwrap();
        assert!(!bytes1.is_empty());

        // Decode generated JPEG thumbnail and check dimensions
        let decoded = image::load_from_memory(&bytes1).unwrap();
        assert_eq!(decoded.width(), 420);
        assert_eq!(decoded.height(), 210);

        // Second render should return the same Arc from memory cache
        let bytes2 = service.render(&path_str).unwrap();
        assert!(Arc::ptr_eq(&bytes1, &bytes2));
    }

    #[test]
    fn test_thumb_service_forbidden() {
        let service = ThumbService::new(420);
        let res = service.render("C:\\forbidden\\secret.exe");
        assert!(matches!(res, Err(ThumbError::Forbidden(_))));
    }
}
