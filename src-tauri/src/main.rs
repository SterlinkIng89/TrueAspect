// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod cropper;
pub mod scanner;
pub mod thumbs;

use std::sync::Arc;
use tauri::http::{header, Response};
use thumbs::ThumbService;

fn main() {
    let thumb_service = Arc::new(ThumbService::new(420));
    let thumb_service_clone = Arc::clone(&thumb_service);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(thumb_service)
        .register_asynchronous_uri_scheme_protocol("thumb", move |_app, request, responder| {
            let service = Arc::clone(&thumb_service_clone);
            std::thread::spawn(move || {
                let uri = request.uri().to_string();
                // URI format: thumb://localhost?path=... or thumb://path=...
                let query = request.uri().query().unwrap_or("");
                let mut path_param = String::new();
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        if k == "path" {
                            path_param = urlencoding::decode(v)
                                .unwrap_or_default()
                                .into_owned();
                            break;
                        }
                    }
                }

                if path_param.is_empty() {
                    // Fallback to decode the entire path from URL if not in query
                    let raw_path = uri.trim_start_matches("thumb://").trim_start_matches("localhost/");
                    path_param = urlencoding::decode(raw_path)
                        .unwrap_or_default()
                        .into_owned();
                }

                match service.render(&path_param) {
                    Ok(data) => {
                        let response = Response::builder()
                            .header(header::CONTENT_TYPE, "image/jpeg")
                            .header(header::CACHE_CONTROL, "public, max-age=86400")
                            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                            .body(data.as_ref().clone())
                            .unwrap();
                        responder.respond(response);
                    }
                    Err(_) => {
                        let response = Response::builder()
                            .status(404)
                            .header(header::CONTENT_TYPE, "text/plain")
                            .body(b"Not Found".to_vec())
                            .unwrap();
                        responder.respond(response);
                    }
                }
            });
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_default_output_dir,
            commands::load_screenshots,
            commands::crop_screenshots,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
