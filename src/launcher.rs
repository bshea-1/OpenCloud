use std::process::Command;

pub fn launch_app(app_id: &str, file_path: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let capitalized = match app_id {
            "photocraft" => "PhotoCraft",
            "vectorcraft" => "VectorCraft",
            "filmcraft" => "FilmCraft",
            "lightcraft" => "LightCraft",
            "pdfcraft" => "PdfCraft",
            "effectcraft" => "EffectCraft",
            "designcraft" => "DesignCraft",
            "soundcraft" => "SoundCraft",
            "cadcraft" => "CADCraft",
            "deckcraft" => "DeckCraft",
            "gridcraft" => "GridCraft",
            "wordcraft" => "WordCraft",
            _ => app_id,
        };

        let mut cmd = Command::new("open");
        cmd.args(["-a", capitalized]);

        if let Some(path) = file_path {
            cmd.arg(path);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to launch {}: {}", capitalized, e))?;
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = dirs::data_local_dir().ok_or("Cannot locate LocalAppData")?;
        let exe_path = local_app_data
            .join("Programs")
            .join("OpenCloud")
            .join(app_id)
            .join(format!("{}.exe", app_id));

        let mut cmd = Command::new(exe_path);
        if let Some(path) = file_path {
            cmd.arg(path);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to launch {}: {}", app_id, e))?;
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("Cannot locate home directory")?;
        let bin_path = home.join(".local/bin").join(app_id);

        let mut cmd = Command::new(bin_path);
        if let Some(path) = file_path {
            cmd.arg(path);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to launch {}: {}", app_id, e))?;
    }

    Ok(())
}
