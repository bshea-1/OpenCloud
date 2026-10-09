use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppCategory {
    Photography,
    VectorGraphics,
    VideoMotion,
    PublishingDocs,
    AudioEngineering,
    CadEngineering,
    OfficeProductivity,
}

impl AppCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            AppCategory::Photography => "Photography & Imaging",
            AppCategory::VectorGraphics => "Vector & Illustration",
            AppCategory::VideoMotion => "Video & Motion VFX",
            AppCategory::PublishingDocs => "Publishing & Documents",
            AppCategory::AudioEngineering => "Audio Engineering",
            AppCategory::CadEngineering => "CAD & Drafting",
            AppCategory::OfficeProductivity => "Productivity & Office",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InstallState {
    NotInstalled,
    Installed,
    UpdateAvailable,
    Downloading { percentage: f32, phase: String },
    Installing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAssetInfo {
    pub name: String,
    pub download_url: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteApp {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub category: AppCategory,
    pub description: String,
    pub token: String,
    pub accent_hex: String,
    pub accent_color: [u8; 3],
    pub install_state: InstallState,
    pub installed_version: Option<String>,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub executable_path: Option<String>,
    pub download_asset: Option<ReleaseAssetInfo>,
    pub file_formats: Vec<String>,
    pub details_expanded: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DownloadProgress {
    pub app_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub phase: String,
    pub error: Option<String>,
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub app_id: String,
    pub app_name: String,
    pub modified_time: u64,
    pub size_bytes: u64,
    pub file_extension: String,
}
