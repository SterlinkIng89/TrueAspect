use crate::cropper::{self, CropResult};
use crate::scanner::{self, Screenshot};
use crate::thumbs::ThumbService;
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
pub struct CropProgressPayload {
    pub current: usize,
    pub total: usize,
    pub name: String,
    pub status: String,
}

#[derive(Clone, Serialize)]
pub struct CropDonePayload {
    pub total: usize,
}

#[tauri::command]
pub fn get_default_output_dir() -> String {
    if let Some(pictures) = dirs::picture_dir() {
        return pictures.join("steam-cropped").to_string_lossy().to_string();
    }
    if let Some(home) = dirs::home_dir() {
        return home
            .join("Pictures")
            .join("steam-cropped")
            .to_string_lossy()
            .to_string();
    }
    PathBuf::from("steam-cropped")
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub fn load_screenshots(
    paths: Vec<String>,
    thumb_service: State<'_, Arc<ThumbService>>,
) -> Result<Vec<Screenshot>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let path_bufs: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let results = scanner::scan(&path_bufs);

    let allowed_paths: Vec<String> = results.iter().map(|s| s.path.clone()).collect();
    thumb_service.register_allowed(&allowed_paths);

    Ok(results)
}

#[tauri::command]
pub async fn crop_screenshots(
    app: AppHandle,
    paths: Vec<String>,
    ratio_str: String,
    out_dir_str: String,
) -> Result<Vec<CropResult>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let out_dir = if out_dir_str.trim().is_empty() {
        PathBuf::from(get_default_output_dir())
    } else {
        PathBuf::from(out_dir_str)
    };

    let target_ratio = cropper::parse_ratio(&ratio_str)?;
    let total = paths.len();

    let completed = Arc::new(AtomicUsize::new(0));
    let app_handle = Arc::new(app);

    let num_cpus = num_cpus::get().clamp(1, 4);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus)
        .build()
        .map_err(|e| e.to_string())?;

    let results = pool.install(|| {
        paths
            .par_iter()
            .map(|path_str| {
                let src_path = Path::new(path_str);
                let res = cropper::process(
                    src_path,
                    &out_dir,
                    target_ratio,
                    cropper::DEFAULT_THRESHOLD,
                    95,
                );

                let done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                let filename = src_path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();

                let _ = app_handle.emit(
                    "crop:progress",
                    CropProgressPayload {
                        current: done,
                        total,
                        name: filename,
                        status: "processing".to_string(),
                    },
                );

                res
            })
            .collect::<Vec<CropResult>>()
    });

    let _ = app_handle.emit("crop:done", CropDonePayload { total });

    Ok(results)
}
