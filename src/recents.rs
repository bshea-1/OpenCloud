use crate::models::RecentProject;
use std::path::PathBuf;
use std::time::SystemTime;

pub fn get_recent_projects() -> Vec<RecentProject> {
    let mut projects = Vec::new();
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return projects,
    };

    let target_dirs = vec![
        home.join("Documents"),
        home.join("Desktop"),
        home.join("Pictures"),
        home.join("Downloads"),
    ];

    for dir in target_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(project) = match_project_file(&path) {
                        projects.push(project);
                    }
                }
            }
        }
    }

    // Sort by modified_time descending
    projects.sort_by(|a, b| b.modified_time.cmp(&a.modified_time));
    projects.truncate(20);

    projects
}

fn match_project_file(path: &PathBuf) -> Option<RecentProject> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())?;

    let (app_id, app_name) = match ext.as_str() {
        "psd" | "png" | "jpg" | "jpeg" | "webp" => ("photocraft", "PhotoCraft"),
        "svg" | "ai" | "eps" => ("vectorcraft", "VectorCraft"),
        "raw" | "dng" | "cr2" | "nef" | "arw" => ("lightcraft", "LightCraft"),
        "pdf" => ("pdfcraft", "PdfCraft"),
        "mp4" | "mov" | "mkv" => ("filmcraft", "FilmCraft"),
        "wav" | "flac" | "mp3" | "aiff" => ("soundcraft", "SoundCraft"),
        "dxf" | "dwg" => ("cadcraft", "CadCraft"),
        "docx" | "md" | "txt" => ("wordcraft", "WordCraft"),
        "xlsx" | "csv" => ("gridcraft", "GridCraft"),
        "pptx" => ("deckcraft", "DeckCraft"),
        _ => return None,
    };

    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    Some(RecentProject {
        path: path.to_string_lossy().to_string(),
        name,
        app_id: app_id.to_string(),
        app_name: app_name.to_string(),
        modified_time: modified,
        size_bytes: metadata.len(),
        file_extension: ext,
    })
}
