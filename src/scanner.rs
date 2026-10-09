use std::path::PathBuf;

pub struct LocalAppInfo {
    pub is_installed: bool,
    pub version: Option<String>,
    pub executable_path: Option<String>,
}

/// Detect whether an app is installed, using a pre-loaded configured directory string.
/// Callers that already have settings loaded should prefer this to avoid redundant disk reads.
pub fn detect_installed_app_with_dir(app_id: &str, configured_dir: &str) -> LocalAppInfo {
    #[cfg(target_os = "macos")]
    {
        let candidates = get_macos_app_names(app_id);
        let mut search_dirs = Vec::new();
        if !configured_dir.is_empty() {
            search_dirs.push(PathBuf::from(configured_dir));
        }
        search_dirs.push(PathBuf::from("/Applications"));
        if let Some(home) = dirs::home_dir() {
            search_dirs.push(home.join("Applications"));
        }

        for name in candidates {
            for dir in &search_dirs {
                let target = dir.join(format!("{}.app", name));
                if target.exists() {
                    let version = read_macos_plist_version(&target);
                    return LocalAppInfo {
                        is_installed: true,
                        version,
                        executable_path: Some(target.to_string_lossy().to_string()),
                    };
                }

                // Also check binary/executable directly
                let bin_target = dir.join(&name);
                if bin_target.is_file() {
                    return LocalAppInfo {
                        is_installed: true,
                        version: None,
                        executable_path: Some(bin_target.to_string_lossy().to_string()),
                    };
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut search_dirs = Vec::new();
        if !configured_dir.is_empty() {
            search_dirs.push(PathBuf::from(configured_dir).join(app_id));
            search_dirs.push(PathBuf::from(configured_dir));
        }
        if let Some(local_app_data) = dirs::data_local_dir() {
            search_dirs.push(local_app_data.join("Programs").join("OpenCloud").join(app_id));
        }
        search_dirs.push(PathBuf::from("C:\\Program Files\\OpenCloud").join(app_id));

        for dir in search_dirs {
            let exe_path = dir.join(format!("{}.exe", app_id));
            if exe_path.exists() {
                return LocalAppInfo {
                    is_installed: true,
                    version: None,
                    executable_path: Some(exe_path.to_string_lossy().to_string()),
                };
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mut search_dirs = Vec::new();
        if !configured_dir.is_empty() {
            search_dirs.push(PathBuf::from(configured_dir));
        }
        if let Some(home) = dirs::home_dir() {
            search_dirs.push(home.join(".local/bin"));
        }
        search_dirs.push(PathBuf::from("/usr/local/bin"));
        search_dirs.push(PathBuf::from("/usr/bin"));

        for dir in search_dirs {
            let bin_path = dir.join(app_id);
            if bin_path.exists() {
                return LocalAppInfo {
                    is_installed: true,
                    version: None,
                    executable_path: Some(bin_path.to_string_lossy().to_string()),
                };
            }
        }
    }

    LocalAppInfo {
        is_installed: false,
        version: None,
        executable_path: None,
    }
}

/// Detect whether an app is installed, loading settings from disk to get the configured directory.
/// Prefer `detect_installed_app_with_dir` when settings are already loaded.
pub fn detect_installed_app(app_id: &str) -> LocalAppInfo {
    let settings = crate::settings::AppSettings::load();
    detect_installed_app_with_dir(app_id, settings.apps.install_directory.trim())
}

#[cfg(target_os = "macos")]
pub fn get_macos_app_names(app_id: &str) -> Vec<String> {
    match app_id {
        "photocraft" => vec!["PhotoCraft".into(), "photocraft".into()],
        "vectorcraft" => vec!["VectorCraft".into(), "vectorcraft".into()],
        "filmcraft" => vec!["FilmCraft".into(), "filmcraft".into()],
        "lightcraft" => vec!["LightCraft".into(), "lightcraft".into()],
        "pdfcraft" => vec!["PdfCraft".into(), "PDFCraft".into(), "pdfcraft".into()],
        "effectcraft" => vec!["EffectCraft".into(), "effectcraft".into()],
        "designcraft" => vec!["DesignCraft".into(), "designcraft".into()],
        "soundcraft" => vec!["SoundCraft".into(), "soundcraft".into()],
        "cadcraft" => vec!["CADCraft".into(), "CadCraft".into(), "cadcraft".into()],
        "deckcraft" => vec!["DeckCraft".into(), "deckcraft".into()],
        "gridcraft" => vec!["GridCraft".into(), "gridcraft".into()],
        "wordcraft" => vec!["WordCraft".into(), "wordcraft".into()],
        _ => vec![app_id.to_string()],
    }
}

#[cfg(target_os = "macos")]
fn read_macos_plist_version(app_bundle: &PathBuf) -> Option<String> {
    let plist_path = app_bundle.join("Contents/Info.plist");
    if !plist_path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&plist_path).ok()?;
    let key = "<key>CFBundleShortVersionString</key>";
    let pos = content.find(key)?;
    // Search only in the text after the key to avoid matching earlier </string> tags
    let tail = &content[pos + key.len()..];
    let s_pos = tail.find("<string>")?;
    // Slice from end of <string> tag and then find the closing tag within that slice
    let after_open = &tail[s_pos + "<string>".len()..];
    let e_pos = after_open.find("</string>")?;
    let ver = after_open[..e_pos].trim();
    if ver.is_empty() {
        return None;
    }
    Some(ver.trim_start_matches('v').to_string())
}
