use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StudioFont {
    pub id: &'static str,
    pub name: &'static str,
    pub family: &'static str,
    pub category: &'static str,
    pub sample_text: &'static str,
    pub file_name: &'static str,
    pub description: &'static str,
}

pub const STUDIO_FONTS: &[StudioFont] = &[
    StudioFont {
        id: "inter",
        name: "Inter Variable",
        family: "Inter",
        category: "Neo-Grotesque Sans",
        sample_text: "The quick brown fox jumps over the lazy dog 1234567890",
        file_name: "Inter-Variable.ttf",
        description: "High-legibility variable typeface crafted for computer screens and precision user interfaces.",
    },
    StudioFont {
        id: "jetbrains-mono",
        name: "JetBrains Mono",
        family: "JetBrains Mono",
        category: "Technical Monospace",
        sample_text: "fn render_studio_layout() -> Result<(), StudioError> { ... }",
        file_name: "JetBrainsMono-Regular.ttf",
        description: "Monospace typeface with code ligatures, distinct glyphs, and high contrast for technical design.",
    },
    StudioFont {
        id: "space-grotesk",
        name: "Space Grotesk",
        family: "Space Grotesk",
        category: "Modern Display Sans",
        sample_text: "CREATIVE ENGINEERING & VECTOR ARCHITECTURE",
        file_name: "SpaceGrotesk-Regular.ttf",
        description: "Proportional sans-serif variant derived from Space Mono with idiosyncratic modernist details.",
    },
    StudioFont {
        id: "outfit",
        name: "Outfit Geometric",
        family: "Outfit",
        category: "Geometric Sans",
        sample_text: "Clean geometric curves and balanced proportions for identity design.",
        file_name: "Outfit-Regular.ttf",
        description: "Modern geometric sans-serif typeface designed for clean branding and editorial layouts.",
    },
    StudioFont {
        id: "syne",
        name: "Syne Editorial",
        family: "Syne",
        category: "Artistic Display",
        sample_text: "AVANT-GARDE TYPOGRAPHY & VISUAL COMPOSITION",
        file_name: "Syne-Regular.ttf",
        description: "Bold, expressive contemporary display typeface originally designed for visual arts centers.",
    },
    StudioFont {
        id: "fira-code",
        name: "Fira Code",
        family: "Fira Code",
        category: "Ligature Monospace",
        sample_text: "0x7F == (tag >> 8) && buffer.len() >= 1024",
        file_name: "FiraCode-Regular.ttf",
        description: "Monospace font with ligatures for common programming multi-character sequences.",
    },
];

pub fn get_system_font_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|h| h.join("Library").join("Fonts"))
            .unwrap_or_else(|| PathBuf::from("/Library/Fonts"))
    }

    #[cfg(target_os = "windows")]
    {
        dirs::data_local_dir()
            .map(|p| p.join("Microsoft").join("Windows").join("Fonts"))
            .unwrap_or_else(|| PathBuf::from("C:\\Windows\\Fonts"))
    }

    #[cfg(target_os = "linux")]
    {
        dirs::data_dir()
            .map(|p| p.join("fonts"))
            .unwrap_or_else(|| PathBuf::from("~/.local/share/fonts"))
    }
}

pub fn is_font_installed(file_name: &str) -> bool {
    let font_dir = get_system_font_dir();
    let font_path = font_dir.join(file_name);
    font_path.exists()
}

pub fn activate_font(font: &StudioFont) -> Result<(), String> {
    // Record the font as "activated" in the OpenCloud config directory.
    // We do NOT write fake/corrupt font binaries to the system font directory,
    // because placeholder bytes would cause every renderer to emit parse errors.
    // The activation manifest is OpenCloud-private; real fonts must be downloaded
    // separately from their upstream sources before they appear in /Library/Fonts.
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Cannot locate config directory".to_string())?
        .join("OpenCloud")
        .join("activated_fonts");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create activation directory: {}", e))?;

    let marker = config_dir.join(format!("{}.activated", font.id));
    if !marker.exists() {
        std::fs::write(&marker, font.name.as_bytes())
            .map_err(|e| format!("Failed to write activation marker for {}: {}", font.name, e))?;
    }

    Ok(())
}

/// Returns true if the given font has a valid font file installed in the system font directory.
#[allow(dead_code)]
pub fn is_real_font_installed(file_name: &str) -> bool {
    let font_dir = get_system_font_dir();
    let p = font_dir.join(file_name);
    // Only count it if the file is non-empty (rejects any zero-byte or stub files)
    p.metadata().map(|m| m.len() > 4096).unwrap_or(false)
}

pub fn deactivate_font(file_name: &str) -> Result<(), String> {
    let font_dir = get_system_font_dir();
    let target_path = font_dir.join(file_name);
    if target_path.exists() {
        std::fs::remove_file(&target_path)
            .map_err(|e| format!("Failed to remove font {}: {}", file_name, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fonts_catalog() {
        assert_eq!(STUDIO_FONTS.len(), 6);
        let ids: Vec<&str> = STUDIO_FONTS.iter().map(|f| f.id).collect();
        assert!(ids.contains(&"inter"));
        assert!(ids.contains(&"jetbrains-mono"));
        assert!(ids.contains(&"space-grotesk"));
        assert!(ids.contains(&"outfit"));
        assert!(ids.contains(&"syne"));
        assert!(ids.contains(&"fira-code"));
    }

    #[test]
    fn test_font_dir_resolution() {
        let font_dir = get_system_font_dir();
        assert!(!font_dir.to_string_lossy().is_empty());
    }
}
