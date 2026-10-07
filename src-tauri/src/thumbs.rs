use image::ImageEncoder;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, RwLock};

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

pub struct Semaphore {
    permits: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            cvar: Condvar::new(),
        }
    }

    pub fn acquire(&self) -> SemaphoreGuard<'_> {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.cvar.wait(permits).unwrap();
        }
        *permits -= 1;
        SemaphoreGuard { sem: self }
    }
}

pub struct SemaphoreGuard<'a> {
    sem: &'a Semaphore,
}

impl<'a> Drop for SemaphoreGuard<'a> {
    fn drop(&mut self) {
        let mut permits = self.sem.permits.lock().unwrap();
        *permits += 1;
        self.sem.cvar.notify_one();
    }
}

pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("true-aspect")
        .join("thumbs")
}

pub struct ThumbService {
    allowed: RwLock<HashSet<String>>,
    cache: Mutex<HashMap<String, Arc<Vec<u8>>>>,
    path_to_keys: Mutex<HashMap<String, HashSet<String>>>,
    thumb_width: u32,
    cache_dir: PathBuf,
    semaphore: Semaphore,
}

impl ThumbService {
    pub fn new(thumb_width: u32) -> Self {
        Self::with_cache_dir(thumb_width, default_cache_dir())
    }

    pub fn with_cache_dir(thumb_width: u32, cache_dir: PathBuf) -> Self {
        let width = if thumb_width == 0 { 420 } else { thumb_width };
        let _ = fs::create_dir_all(&cache_dir);
        let max_concurrent = num_cpus::get().clamp(1, 4);

        Self {
            allowed: RwLock::new(HashSet::new()),
            cache: Mutex::new(HashMap::new()),
            path_to_keys: Mutex::new(HashMap::new()),
            thumb_width: width,
            cache_dir,
            semaphore: Semaphore::new(max_concurrent),
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

    fn compute_cache_key(&self, path_str: &str, metadata: &fs::Metadata) -> String {
        let mod_time = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut hasher = Sha256::new();
        hasher.update(path_str.as_bytes());
        hasher.update(mod_time.to_le_bytes());
        hasher.update(metadata.len().to_le_bytes());
        hasher.update(self.thumb_width.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn evict_cache_key(&self, cache: &mut HashMap<String, Arc<Vec<u8>>>, key: &str) {
        cache.remove(key);
        let disk_path = self.cache_dir.join(format!("{}.jpg", key));
        let _ = fs::remove_file(disk_path);
    }

    pub fn invalidate_paths<I, S>(&self, paths: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut map = self.path_to_keys.lock().unwrap();
        let mut cache = self.cache.lock().unwrap();
        for p in paths {
            let p_str = p.as_ref();
            if let Some(keys) = map.remove(p_str) {
                for key in &keys {
                    self.evict_cache_key(&mut cache, key);
                }
            }

            let path = PathBuf::from(p_str);
            if let Ok(metadata) = fs::metadata(&path) {
                let key = self.compute_cache_key(p_str, &metadata);
                self.evict_cache_key(&mut cache, &key);
            }
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

    fn get_cached(&self, cache_key: &str, disk_path: &Path) -> Option<Arc<Vec<u8>>> {
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(cache_key) {
                return Some(Arc::clone(cached));
            }
        }

        if disk_path.is_file() {
            if let Ok(bytes) = fs::read(disk_path) {
                let arc_data = Arc::new(bytes);
                let mut cache = self.cache.lock().unwrap();
                cache.insert(cache_key.to_string(), Arc::clone(&arc_data));
                return Some(arc_data);
            }
        }

        None
    }

    pub fn render(&self, path_str: &str) -> Result<Arc<Vec<u8>>, ThumbError> {
        let path = PathBuf::from(path_str);
        if !self.is_allowed(&path) {
            return Err(ThumbError::Forbidden(path_str.to_string()));
        }

        let metadata = fs::metadata(&path).map_err(|_| ThumbError::NotFound(path_str.to_string()))?;
        let cache_key = self.compute_cache_key(path_str, &metadata);
        let disk_path = self.cache_dir.join(format!("{}.jpg", cache_key));

        {
            let mut map = self.path_to_keys.lock().unwrap();
            map.entry(path_str.to_string()).or_default().insert(cache_key.clone());
        }

        if let Some(cached) = self.get_cached(&cache_key, &disk_path) {
            return Ok(cached);
        }

        let _permit = self.semaphore.acquire();

        if let Some(cached) = self.get_cached(&cache_key, &disk_path) {
            return Ok(cached);
        }

        let img = image::open(&path).map_err(|e| ThumbError::DecodeError(e.to_string()))?;
        let (orig_w, orig_h) = (img.width(), img.height());

        let target_w = if orig_w < self.thumb_width { orig_w } else { self.thumb_width };
        let target_h = (((orig_h as f64) * (target_w as f64) / (orig_w as f64)).round() as u32).max(1);

        let resized = img.thumbnail(target_w, target_h);

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

        // Write to disk cache
        let _ = fs::write(&disk_path, &buf);

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
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().join("thumb_cache");
        let service = ThumbService::with_cache_dir(420, cache_dir.clone());
        let img_path = dir.path().join("thumb_test.png");

        let mut img = RgbaImage::new(1000, 500);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([100, 150, 200, 255]);
        }
        img.save(&img_path).unwrap();

        let path_str = img_path.to_string_lossy().to_string();
        service.register_allowed([&path_str]);

        // First render: creates disk cache file and memory cache
        let bytes1 = service.render(&path_str).unwrap();
        assert!(!bytes1.is_empty());

        let decoded = image::load_from_memory(&bytes1).unwrap();
        assert_eq!(decoded.width(), 420);
        assert_eq!(decoded.height(), 210);

        // Verify disk cache file was written
        let cached_files: Vec<_> = fs::read_dir(&cache_dir).unwrap().flatten().collect();
        assert_eq!(cached_files.len(), 1);

        // Second render: returns same Arc from memory cache
        let bytes2 = service.render(&path_str).unwrap();
        assert!(Arc::ptr_eq(&bytes1, &bytes2));

        // Create fresh service pointing to same disk cache dir: verifies disk cache hit
        let service2 = ThumbService::with_cache_dir(420, cache_dir);
        service2.register_allowed([&path_str]);
        let bytes3 = service2.render(&path_str).unwrap();
        assert_eq!(bytes1.as_slice(), bytes3.as_slice());
    }

    #[test]
    fn test_thumb_service_forbidden() {
        let service = ThumbService::new(420);
        let res = service.render("C:\\forbidden\\secret.exe");
        assert!(matches!(res, Err(ThumbError::Forbidden(_))));
    }

    #[test]
    fn test_thumb_service_invalidate_paths() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().join("thumb_cache");
        let service = ThumbService::with_cache_dir(420, cache_dir.clone());
        let img_path = dir.path().join("thumb_test.png");

        let mut img = RgbaImage::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([100, 150, 200, 255]);
        }
        img.save(&img_path).unwrap();

        let path_str = img_path.to_string_lossy().to_string();
        service.register_allowed([&path_str]);

        // Render once
        let bytes1 = service.render(&path_str).unwrap();
        assert!(!bytes1.is_empty());

        let cached_files_before: Vec<_> = fs::read_dir(&cache_dir).unwrap().flatten().collect();
        assert_eq!(cached_files_before.len(), 1);

        // Invalidate path
        service.invalidate_paths([&path_str]);

        let cached_files_after: Vec<_> = fs::read_dir(&cache_dir).unwrap().flatten().collect();
        assert_eq!(cached_files_after.len(), 0);

        // Modify image
        let mut img2 = RgbaImage::new(50, 50);
        for pixel in img2.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        img2.save(&img_path).unwrap();

        // Render again after modification
        let bytes2 = service.render(&path_str).unwrap();
        let decoded = image::load_from_memory(&bytes2).unwrap();
        assert_eq!(decoded.width(), 50);
        assert_eq!(decoded.height(), 50);
    }

    #[test]
    fn test_semaphore_permits() {
        let sem = Arc::new(Semaphore::new(2));
        let g1 = sem.acquire();
        let g2 = sem.acquire();
        drop(g1);
        let _g3 = sem.acquire();
        drop(g2);
    }
}
