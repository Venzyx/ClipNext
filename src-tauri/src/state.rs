use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::db::{Db, Limits};

pub const DEFAULT_SHORTCUT: &str = "Control+Shift+KeyV";

pub struct AppState {
    pub db: Mutex<Db>,
    pub data_dir: PathBuf,
    /// Hash of the content we just put on the clipboard ourselves, so the
    /// watcher does not record our own "copy back" as a new item.
    suppressed: Mutex<Option<(String, Instant)>>,
    /// Window that had focus when the panel was opened (paste destination).
    paste_target: Mutex<Option<isize>>,
}

impl AppState {
    pub fn new(db: Db, data_dir: PathBuf) -> Self {
        Self {
            db: Mutex::new(db),
            data_dir,
            suppressed: Mutex::new(None),
            paste_target: Mutex::new(None),
        }
    }

    pub fn images_dir(&self) -> PathBuf {
        self.data_dir.join("images")
    }

    pub fn set_paste_target(&self, target: Option<isize>) {
        *self.paste_target.lock().unwrap() = target;
    }

    pub fn take_paste_target(&self) -> Option<isize> {
        self.paste_target.lock().unwrap().take()
    }

    pub fn suppress(&self, hash: String) {
        *self.suppressed.lock().unwrap() = Some((hash, Instant::now()));
    }

    pub fn take_suppressed(&self, hash: &str) -> bool {
        let mut guard = self.suppressed.lock().unwrap();
        match guard.as_ref() {
            Some((h, at)) if h == hash && at.elapsed() < Duration::from_secs(3) => {
                *guard = None;
                true
            }
            _ => false,
        }
    }

    /// Deletes files from disk, but only inside the images directory.
    pub fn remove_files(&self, files: &[PathBuf]) {
        let dir = self.images_dir();
        for f in files {
            if f.starts_with(&dir) {
                let _ = fs::remove_file(f);
            }
        }
    }
}

/// Removes image files on disk that no database row refers to.
pub fn sweep_orphans(db: &Db, images_dir: &Path) {
    let Ok(referenced) = db.referenced_files() else {
        return;
    };
    let Ok(entries) = fs::read_dir(images_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && !referenced.contains(&path) {
            let _ = fs::remove_file(path);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub max_items: i64,
    pub max_image_mb: i64,
    pub save_images: bool,
    pub auto_paste: bool,
    pub language: String,
    pub theme: String,
    pub shortcut: String,
    pub autostart: bool,
}

impl Settings {
    pub fn load(db: &Db) -> Self {
        let get = |k: &str| db.get_setting(k).ok().flatten();
        let int = |k: &str, default: i64, lo: i64, hi: i64| {
            get(k)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(default)
                .clamp(lo, hi)
        };
        let flag = |k: &str, default: bool| {
            get(k).map(|v| v == "true").unwrap_or(default)
        };
        let language = get("language")
            .filter(|l| matches!(l.as_str(), "system" | "tr" | "en"))
            .unwrap_or_else(|| "system".to_string());
        Self {
            max_items: int("max_items", 500, 20, 5000),
            max_image_mb: int("max_image_mb", 200, 10, 5000),
            save_images: flag("save_images", true),
            auto_paste: flag("auto_paste", true),
            language,
            theme: get("theme")
                .filter(|t| matches!(t.as_str(), "system" | "light" | "dark"))
                .unwrap_or_else(|| "system".to_string()),
            shortcut: get("shortcut")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string()),
            autostart: flag("autostart", false),
        }
    }

    pub fn limits(&self) -> Limits {
        Limits {
            max_items: self.max_items,
            max_image_bytes: self.max_image_mb * 1024 * 1024,
        }
    }
}

/// Validates and stores one setting.
pub fn write_setting(db: &mut Db, key: &str, value: &str) -> Result<(), String> {
    let valid = match key {
        "max_items" | "max_image_mb" => value.parse::<i64>().is_ok(),
        "save_images" | "autostart" | "auto_paste" => matches!(value, "true" | "false"),
        "language" => matches!(value, "system" | "tr" | "en"),
        "theme" => matches!(value, "system" | "light" | "dark"),
        "shortcut" => !value.trim().is_empty() && value.len() <= 64,
        _ => false,
    };
    if !valid {
        return Err(format!("invalid setting: {key}={value}"));
    }
    db.set_setting(key, value).map_err(|e| e.to_string())
}
