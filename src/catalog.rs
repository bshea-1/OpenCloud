use crate::models::{AppCategory, InstallState, ReleaseAssetInfo, SuiteApp};
use crate::scanner::detect_installed_app_with_dir;
use crate::settings::AppSettings;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde_json::Value;
use std::time::Duration;

pub struct CatalogDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub repo: &'static str,
    pub category: AppCategory,
    pub description: &'static str,
    pub token: &'static str,
    pub accent_hex: &'static str,
    pub accent_color: [u8; 3],
    pub file_formats: &'static [&'static str],
}

pub const APP_CATALOG: &[CatalogDefinition] = &[
    CatalogDefinition {
        id: "photocraft",
        name: "PhotoCraft",
        repo: "storytold/photocraft",
        category: AppCategory::Photography,
        description: "Used for editing and retouching photos, graphic design, digital painting, and layer compositing.",
        token: "Pc",
        accent_hex: "#00d2d3",
        accent_color: [0, 210, 211],
        file_formats: &[".psd", ".raw", ".png", ".jpg", ".webp", ".tiff"],
    },
    CatalogDefinition {
        id: "vectorcraft",
        name: "VectorCraft",
        repo: "storytold/vectorcraft",
        category: AppCategory::VectorGraphics,
        description: "Used for designing logos, vector illustrations, icons, Bezier curves, and scalable typography.",
        token: "Vc",
        accent_hex: "#ff9f43",
        accent_color: [255, 159, 67],
        file_formats: &[".svg", ".ai", ".eps", ".pdf"],
    },
    CatalogDefinition {
        id: "filmcraft",
        name: "FilmCraft",
        repo: "storytold/filmcraft",
        category: AppCategory::VideoMotion,
        description: "Used for editing multi-track video projects, color grading, audio trimming, and video production.",
        token: "Fc",
        accent_hex: "#a55eea",
        accent_color: [165, 94, 234],
        file_formats: &[".mp4", ".mov", ".mkv", ".webm"],
    },
    CatalogDefinition {
        id: "lightcraft",
        name: "LightCraft",
        repo: "storytold/lightcraft",
        category: AppCategory::Photography,
        description: "Used for organizing photo libraries, processing raw camera files, and applying tone adjustments.",
        token: "Lc",
        accent_hex: "#54a0ff",
        accent_color: [84, 160, 255],
        file_formats: &[".raw", ".dng", ".cr2", ".nef", ".arw"],
    },
    CatalogDefinition {
        id: "pdfcraft",
        name: "PdfCraft",
        repo: "storytold/pdfcraft",
        category: AppCategory::PublishingDocs,
        description: "Used for viewing, editing, converting, merging, and digitally signing PDF documents and forms.",
        token: "Pd",
        accent_hex: "#ee5253",
        accent_color: [238, 82, 83],
        file_formats: &[".pdf", ".xfdf", ".fdf"],
    },
    CatalogDefinition {
        id: "effectcraft",
        name: "EffectCraft",
        repo: "storytold/effectcraft",
        category: AppCategory::VideoMotion,
        description: "Used for creating visual effects (VFX), motion graphic animations, titles, and compositing.",
        token: "Ec",
        accent_hex: "#5f27cd",
        accent_color: [95, 39, 205],
        file_formats: &[".aep", ".json", ".mp4", ".mov"],
    },
    CatalogDefinition {
        id: "designcraft",
        name: "DesignCraft",
        repo: "storytold/designcraft",
        category: AppCategory::PublishingDocs,
        description: "Used for designing print and digital publications, books, brochures, flyers, and magazines.",
        token: "Dc",
        accent_hex: "#ff6b81",
        accent_color: [255, 107, 129],
        file_formats: &[".indd", ".idml", ".pdf", ".epub"],
    },
    CatalogDefinition {
        id: "soundcraft",
        name: "SoundCraft",
        repo: "storytold/soundcraft",
        category: AppCategory::AudioEngineering,
        description: "Used for recording podcasts, mixing audio tracks, sound editing, and audio mastering.",
        token: "Sc",
        accent_hex: "#10ac84",
        accent_color: [16, 172, 132],
        file_formats: &[".wav", ".mp3", ".flac", ".aiff"],
    },
    CatalogDefinition {
        id: "cadcraft",
        name: "CadCraft",
        repo: "storytold/cadcraft",
        category: AppCategory::CadEngineering,
        description: "Used for 2D technical drawings, architectural blueprints, floor plans, and 3D CAD modeling.",
        token: "Cd",
        accent_hex: "#48dbfb",
        accent_color: [72, 219, 251],
        file_formats: &[".dxf", ".dwg", ".step", ".obj"],
    },
    CatalogDefinition {
        id: "deckcraft",
        name: "DeckCraft",
        repo: "storytold/deckcraft",
        category: AppCategory::OfficeProductivity,
        description: "Used for building and delivering presentation slide decks, pitch presentations, and pitch decks.",
        token: "Dk",
        accent_hex: "#feca57",
        accent_color: [254, 202, 87],
        file_formats: &[".pptx", ".ppt", ".pdf"],
    },
    CatalogDefinition {
        id: "gridcraft",
        name: "GridCraft",
        repo: "storytold/gridcraft",
        category: AppCategory::OfficeProductivity,
        description: "Used for building financial models, calculating formulas, tracking budgets, and spreadsheet analysis.",
        token: "Gc",
        accent_hex: "#1dd1a1",
        accent_color: [29, 209, 161],
        file_formats: &[".xlsx", ".xls", ".csv"],
    },
    CatalogDefinition {
        id: "wordcraft",
        name: "WordCraft",
        repo: "storytold/wordcraft",
        category: AppCategory::OfficeProductivity,
        description: "Used for writing essays, articles, manuscripts, documentation, and formatted text documents.",
        token: "Wc",
        accent_hex: "#2e86de",
        accent_color: [46, 134, 222],
        file_formats: &[".docx", ".doc", ".md", ".txt"],
    },
];

fn get_fallback_asset(app_id: &str) -> (Option<String>, Option<ReleaseAssetInfo>) {
    #[cfg(target_os = "macos")]
    {
        match app_id {
            "photocraft" => (
                Some("0.5.0".to_string()),
                Some(ReleaseAssetInfo {
                    name: "photocraft-0.5.0-macos-universal.dmg".into(),
                    download_url: "https://github.com/storytold/photocraft/releases/download/v0.5.0/photocraft-0.5.0-macos-universal.dmg".into(),
                    size_bytes: 74849198,
                }),
            ),
            "vectorcraft" => (
                Some("0.7.0".to_string()),
                Some(ReleaseAssetInfo {
                    name: "vectorcraft-0.7.0-macos-universal.dmg".into(),
                    download_url: "https://github.com/storytold/vectorcraft/releases/download/v0.7.0/vectorcraft-0.7.0-macos-universal.dmg".into(),
                    size_bytes: 85348352,
                }),
            ),
            _ => (None, None),
        }
    }
    #[cfg(target_os = "windows")]
    {
        match app_id {
            "photocraft" => (
                Some("0.5.0".to_string()),
                Some(ReleaseAssetInfo {
                    name: "photocraft-0.5.0-windows-x64-portable.zip".into(),
                    download_url: "https://github.com/storytold/photocraft/releases/download/v0.5.0/photocraft-0.5.0-windows-x64-portable.zip".into(),
                    size_bytes: 78000000,
                }),
            ),
            _ => (None, None),
        }
    }
    #[cfg(target_os = "linux")]
    {
        match app_id {
            "photocraft" => (
                Some("0.5.0".to_string()),
                Some(ReleaseAssetInfo {
                    name: "photocraft-0.5.0-linux-x86_64.AppImage".into(),
                    download_url: "https://github.com/storytold/photocraft/releases/download/v0.5.0/photocraft-0.5.0-linux-x86_64.AppImage".into(),
                    size_bytes: 74000000,
                }),
            ),
            _ => (None, None),
        }
    }
}

pub fn get_initial_apps() -> Vec<SuiteApp> {
    // Load settings once and reuse the install directory for all apps.
    let settings = AppSettings::load();
    let install_dir = settings.apps.install_directory.clone();
    let install_dir = install_dir.trim();

    APP_CATALOG
        .iter()
        .map(|def| {
            let local_info = detect_installed_app_with_dir(def.id, install_dir);
            let (fallback_ver, fallback_asset) = get_fallback_asset(def.id);
            let install_state = if local_info.is_installed {
                InstallState::Installed
            } else {
                InstallState::NotInstalled
            };

            SuiteApp {
                id: def.id.to_string(),
                name: def.name.to_string(),
                repo: def.repo.to_string(),
                category: def.category,
                description: def.description.to_string(),
                token: def.token.to_string(),
                accent_hex: def.accent_hex.to_string(),
                accent_color: def.accent_color,
                install_state,
                installed_version: local_info.version,
                latest_version: fallback_ver,
                release_notes: None,
                published_at: None,
                executable_path: local_info.executable_path,
                download_asset: fallback_asset,
                file_formats: def.file_formats.iter().map(|s| s.to_string()).collect(),
                details_expanded: false,
            }
        })
        .collect()
}

pub async fn refresh_remote_releases(apps: &mut [SuiteApp]) {
    // Load settings exactly once for the entire refresh cycle.
    let settings = AppSettings::load();
    let install_dir = settings.apps.install_directory.clone();
    let install_dir_str = install_dir.trim();

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("OpenCloud-Desktop/1.0.0"),
    );

    let auth_token = if !settings.network.github_token.trim().is_empty() {
        Some(settings.network.github_token.trim().to_string())
    } else if let Ok(env_tok) = std::env::var("GITHUB_TOKEN") {
        Some(env_tok)
    } else {
        std::process::Command::new("gh")
            .arg("auth")
            .arg("token")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
                } else {
                    None
                }
            })
    };

    if let Some(tok) = auth_token {
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", tok)) {
            headers.insert(AUTHORIZATION, val);
        }
    }

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(settings.network.api_timeout_secs))
        .build()
        .unwrap_or_default();

    for app in apps.iter_mut() {
        if let Ok(json) = fetch_github_release(&client, &app.repo).await {
            let (tag, notes, published, chosen_asset) = parse_release_payload(&json);
            app.latest_version = tag.clone();
            app.release_notes = notes;
            app.published_at = published;
            app.download_asset = chosen_asset;

            // Reuse cached install_dir_str -- no extra disk read per app.
            let local_info = detect_installed_app_with_dir(&app.id, install_dir_str);
            app.installed_version = local_info.version.clone();
            app.executable_path = local_info.executable_path;

            if local_info.is_installed {
                if let (Some(curr), Some(latest)) = (&local_info.version, &tag) {
                    if is_newer_version(latest, curr) {
                        app.install_state = InstallState::UpdateAvailable;
                    } else {
                        app.install_state = InstallState::Installed;
                    }
                } else {
                    app.install_state = InstallState::Installed;
                }
            } else {
                app.install_state = InstallState::NotInstalled;
            }
        } else {
            // Graceful fallback when offline or when remote GitHub release is pending
            let local_info = detect_installed_app_with_dir(&app.id, install_dir_str);
            app.installed_version = local_info.version.clone();
            app.executable_path = local_info.executable_path;
            if app.latest_version.is_none() {
                app.latest_version = Some("0.1.0".into());
            }
            if app.release_notes.is_none() {
                app.release_notes = Some(format!(
                    "Release Highlights for {}:\n- Pure-Rust computational and rendering core\n- Hardware accelerated wgpu rendering pipeline\n- Native platform integration and lossless file format support",
                    app.name
                ));
            }
            if local_info.is_installed {
                app.install_state = InstallState::Installed;
            } else {
                app.install_state = InstallState::NotInstalled;
            }
        }
    }
}

async fn fetch_github_release(client: &reqwest::Client, repo: &str) -> Result<Value, String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API returned: {}", resp.status()));
    }

    resp.json::<Value>()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))
}

fn parse_release_payload(
    json: &Value,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<ReleaseAssetInfo>,
) {
    let tag = json["tag_name"]
        .as_str()
        .map(|s| s.trim_start_matches('v').to_string());
    let notes = json["body"].as_str().map(|s| s.to_string());
    let published = json["published_at"].as_str().map(|s| s.to_string());

    let mut chosen_asset: Option<ReleaseAssetInfo> = None;

    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or_default();
            let download_url = asset["browser_download_url"].as_str().unwrap_or_default();
            let size = asset["size"].as_u64().unwrap_or_default();

            #[cfg(target_os = "macos")]
            {
                if name.ends_with("macos-universal.dmg") || name.ends_with(".dmg") {
                    chosen_asset = Some(ReleaseAssetInfo {
                        name: name.to_string(),
                        download_url: download_url.to_string(),
                        size_bytes: size,
                    });
                    break;
                }
            }

            #[cfg(target_os = "windows")]
            {
                if name.ends_with("windows-x64-portable.zip")
                    || name.ends_with("windows-x64.msi")
                    || (name.contains("windows") && name.ends_with(".zip"))
                {
                    chosen_asset = Some(ReleaseAssetInfo {
                        name: name.to_string(),
                        download_url: download_url.to_string(),
                        size_bytes: size,
                    });
                    break;
                }
            }

            #[cfg(target_os = "linux")]
            {
                if name.ends_with("linux-x86_64.AppImage")
                    || name.ends_with(".AppImage")
                    || name.ends_with("linux-x86_64.tar.gz")
                {
                    chosen_asset = Some(ReleaseAssetInfo {
                        name: name.to_string(),
                        download_url: download_url.to_string(),
                        size_bytes: size,
                    });
                    break;
                }
            }
        }
    }

    (tag, notes, published, chosen_asset)
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    let clean_latest = latest.trim_start_matches('v');
    let clean_current = current.trim_start_matches('v');

    if let (Ok(v_latest), Ok(v_curr)) = (
        semver::Version::parse(clean_latest),
        semver::Version::parse(clean_current),
    ) {
        v_latest > v_curr
    } else {
        clean_latest != clean_current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_completeness() {
        assert_eq!(APP_CATALOG.len(), 12);
        let ids: Vec<&str> = APP_CATALOG.iter().map(|c| c.id).collect();
        assert!(ids.contains(&"photocraft"));
        assert!(ids.contains(&"vectorcraft"));
        assert!(ids.contains(&"filmcraft"));
        assert!(ids.contains(&"soundcraft"));
        assert!(ids.contains(&"cadcraft"));
        assert!(ids.contains(&"wordcraft"));
        assert!(ids.contains(&"gridcraft"));
        assert!(ids.contains(&"deckcraft"));
        assert!(ids.contains(&"pdfcraft"));
        assert!(ids.contains(&"lightcraft"));
        assert!(ids.contains(&"effectcraft"));
        assert!(ids.contains(&"designcraft"));
    }

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.6.0", "0.5.0"));
        assert!(is_newer_version("v1.0.0", "0.9.9"));
        assert!(!is_newer_version("0.5.0", "0.5.0"));
        assert!(!is_newer_version("0.4.0", "0.5.0"));
    }
}
