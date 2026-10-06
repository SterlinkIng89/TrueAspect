use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Screenshot {
    pub path: String,
    pub name: String,
}

pub fn is_supported_extension(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            let lower = ext.to_lowercase();
            lower == "jpg" || lower == "jpeg" || lower == "png"
        }
        None => false,
    }
}

pub fn scan<P: AsRef<Path>>(paths: &[P]) -> Vec<Screenshot> {
    let mut seen = HashSet::new();
    let mut screenshots = Vec::new();

    let mut add_file = |file_path: &Path| {
        let canonical_str = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.to_path_buf())
            .to_string_lossy()
            .to_string();

        if seen.insert(canonical_str) && is_supported_extension(file_path) {
            let name = file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            screenshots.push(Screenshot {
                path: file_path.to_string_lossy().to_string(),
                name,
            });
        }
    };

    for p in paths {
        let path = p.as_ref();
        if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        add_file(&entry_path);
                    }
                }
            }
        } else if path.is_file() {
            add_file(path);
        }
    }

    screenshots.sort_by_key(|a| a.name.to_lowercase());
    screenshots
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_scan_files_and_dirs() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("b_screenshot.jpg");
        let f2 = dir.path().join("a_screenshot.PNG");
        let f3 = dir.path().join("ignore.txt");

        File::create(&f1).unwrap();
        File::create(&f2).unwrap();
        File::create(&f3).unwrap();

        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        let f4 = sub.join("c_screenshot.jpeg");
        File::create(&f4).unwrap();

        // Scan folder
        let results = scan(&[dir.path()]);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "a_screenshot.PNG");
        assert_eq!(results[1].name, "b_screenshot.jpg");

        // Scan specific file and folder deduplication
        let mixed = scan(&[dir.path(), &f1, &f4]);
        assert_eq!(mixed.len(), 3);
        assert_eq!(mixed[0].name, "a_screenshot.PNG");
        assert_eq!(mixed[1].name, "b_screenshot.jpg");
        assert_eq!(mixed[2].name, "c_screenshot.jpeg");
    }
}
