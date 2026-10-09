use std::path::{Path, PathBuf};
use std::process::Command;

pub fn install_package(app_id: &str, archive_path: &Path) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        install_macos_package(app_id, archive_path)
    }

    #[cfg(target_os = "windows")]
    {
        install_windows_package(app_id, archive_path)
    }

    #[cfg(target_os = "linux")]
    {
        install_linux_package(app_id, archive_path)
    }
}

#[cfg(target_os = "macos")]
fn get_target_macos_install_dir() -> PathBuf {
    let settings = crate::settings::AppSettings::load();
    let configured_dir = settings.apps.install_directory.trim();
    if !configured_dir.is_empty() {
        let p = PathBuf::from(configured_dir);
        let _ = std::fs::create_dir_all(&p);
        p
    } else {
        let test_file = PathBuf::from("/Applications/.opencloud_write_test");
        if std::fs::write(&test_file, b"").is_ok() {
            let _ = std::fs::remove_file(&test_file);
            PathBuf::from("/Applications")
        } else {
            let user_apps = dirs::home_dir().unwrap_or_default().join("Applications");
            let _ = std::fs::create_dir_all(&user_apps);
            user_apps
        }
    }
}

#[cfg(target_os = "macos")]
fn install_macos_package(app_id: &str, pkg_path: &Path) -> Result<String, String> {
    let file_str = pkg_path.to_string_lossy().to_string();
    let target_dest_dir = get_target_macos_install_dir();

    if file_str.ends_with(".dmg") {
        install_macos_dmg(app_id, pkg_path, &target_dest_dir)
    } else if file_str.ends_with(".zip") {
        install_macos_zip(app_id, pkg_path, &target_dest_dir)
    } else if file_str.ends_with(".tar.gz") || file_str.ends_with(".tgz") {
        install_macos_tar(app_id, pkg_path, &target_dest_dir)
    } else if pkg_path.is_dir() && file_str.ends_with(".app") {
        let app_name = pkg_path.file_name().unwrap();
        let target_dest = target_dest_dir.join(app_name);
        if target_dest.exists() {
            let _ = std::fs::remove_dir_all(&target_dest);
        }
        let cp_status = Command::new("cp")
            .args(["-R", pkg_path.to_str().unwrap(), target_dest.to_str().unwrap()])
            .status()
            .map_err(|e| format!("Failed to copy app bundle: {}", e))?;
        if !cp_status.success() {
            return Err("Failed to copy app bundle".into());
        }
        Ok(target_dest.to_string_lossy().to_string())
    } else {
        // Raw executable binary
        let dest_bin = target_dest_dir.join(app_id);
        std::fs::copy(pkg_path, &dest_bin).map_err(|e| format!("Failed to copy binary: {}", e))?;
        let _ = Command::new("chmod").args(["+x", dest_bin.to_str().unwrap()]).status();
        Ok(dest_bin.to_string_lossy().to_string())
    }
}

#[cfg(target_os = "macos")]
fn install_macos_dmg(app_id: &str, dmg_path: &Path, target_dest_dir: &Path) -> Result<String, String> {
    let mount_dir = std::env::temp_dir().join(format!("opencloud_mount_{}", app_id));
    let _ = std::fs::remove_dir_all(&mount_dir);
    std::fs::create_dir_all(&mount_dir)
        .map_err(|e| format!("Failed to create mount directory: {}", e))?;

    let attach_status = Command::new("hdiutil")
        .args([
            "attach",
            dmg_path.to_str().unwrap(),
            "-mountpoint",
            mount_dir.to_str().unwrap(),
            "-nobrowse",
            "-quiet",
        ])
        .status()
        .map_err(|e| format!("hdiutil attach failed: {}", e))?;

    if !attach_status.success() {
        return Err("hdiutil attach failed to mount disk image".into());
    }

    let mut found_app: Option<PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&mount_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("app") {
                found_app = Some(path);
                break;
            }
        }
    }

    let app_bundle = match found_app {
        Some(b) => b,
        None => {
            let _ = Command::new("hdiutil")
                .args(["detach", mount_dir.to_str().unwrap(), "-quiet"])
                .status();
            return Err("No .app bundle found inside mounted disk image".to_string());
        }
    };

    let app_name = app_bundle.file_name().unwrap();
    let target_dest = target_dest_dir.join(app_name);

    if target_dest.exists() {
        let _ = std::fs::remove_dir_all(&target_dest);
    }

    let cp_status = Command::new("cp")
        .args(["-R", app_bundle.to_str().unwrap(), target_dest.to_str().unwrap()])
        .status()
        .map_err(|e| format!("Failed to copy app bundle: {}", e))?;

    if !cp_status.success() {
        let _ = Command::new("hdiutil")
            .args(["detach", mount_dir.to_str().unwrap(), "-quiet"])
            .status();
        return Err("Failed to copy app into install location".into());
    }

    // Strip quarantine flag
    let _ = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine", target_dest.to_str().unwrap()])
        .status();

    let _ = Command::new("hdiutil")
        .args(["detach", mount_dir.to_str().unwrap(), "-quiet"])
        .status();
    let _ = std::fs::remove_dir_all(&mount_dir);

    Ok(target_dest.to_string_lossy().to_string())
}

#[cfg(target_os = "macos")]
fn install_macos_zip(app_id: &str, zip_path: &Path, target_dest_dir: &Path) -> Result<String, String> {
    let extract_dir = std::env::temp_dir().join(format!("opencloud_zip_{}", app_id));
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    let status = Command::new("unzip")
        .args(["-q", "-o", zip_path.to_str().unwrap(), "-d", extract_dir.to_str().unwrap()])
        .status()
        .map_err(|e| format!("unzip failed: {}", e))?;

    if !status.success() {
        return Err("unzip returned non-zero status".into());
    }

    let mut found_app: Option<PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&extract_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("app") {
                found_app = Some(path);
                break;
            }
        }
    }

    if let Some(app_bundle) = found_app {
        let app_name = app_bundle.file_name().unwrap();
        let target_dest = target_dest_dir.join(app_name);
        if target_dest.exists() {
            let _ = std::fs::remove_dir_all(&target_dest);
        }
        let cp_status = Command::new("cp")
            .args(["-R", app_bundle.to_str().unwrap(), target_dest.to_str().unwrap()])
            .status()
            .map_err(|e| format!("Failed to copy extracted app: {}", e))?;
        let _ = std::fs::remove_dir_all(&extract_dir);
        if !cp_status.success() {
            return Err("Failed to copy extracted app bundle".into());
        }
        let _ = Command::new("xattr")
            .args(["-dr", "com.apple.quarantine", target_dest.to_str().unwrap()])
            .status();
        Ok(target_dest.to_string_lossy().to_string())
    } else {
        let _ = std::fs::remove_dir_all(&extract_dir);
        Err("No .app found inside zip archive".into())
    }
}

#[cfg(target_os = "macos")]
fn install_macos_tar(app_id: &str, tar_path: &Path, target_dest_dir: &Path) -> Result<String, String> {
    let extract_dir = std::env::temp_dir().join(format!("opencloud_tar_{}", app_id));
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    let status = Command::new("tar")
        .args(["-xzf", tar_path.to_str().unwrap(), "-C", extract_dir.to_str().unwrap()])
        .status()
        .map_err(|e| format!("tar failed: {}", e))?;

    if !status.success() {
        return Err("tar returned non-zero status".into());
    }

    let mut found_app: Option<PathBuf> = None;
    if let Ok(entries) = std::fs::read_dir(&extract_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("app") {
                found_app = Some(path);
                break;
            }
        }
    }

    if let Some(app_bundle) = found_app {
        let app_name = app_bundle.file_name().unwrap();
        let target_dest = target_dest_dir.join(app_name);
        if target_dest.exists() {
            let _ = std::fs::remove_dir_all(&target_dest);
        }
        let cp_status = Command::new("cp")
            .args(["-R", app_bundle.to_str().unwrap(), target_dest.to_str().unwrap()])
            .status()
            .map_err(|e| format!("Failed to copy extracted app: {}", e))?;
        let _ = std::fs::remove_dir_all(&extract_dir);
        if !cp_status.success() {
            return Err("Failed to copy extracted app bundle".into());
        }
        let _ = Command::new("xattr")
            .args(["-dr", "com.apple.quarantine", target_dest.to_str().unwrap()])
            .status();
        Ok(target_dest.to_string_lossy().to_string())
    } else {
        let _ = std::fs::remove_dir_all(&extract_dir);
        Err("No .app found inside tar archive".into())
    }
}

#[cfg(target_os = "windows")]
fn install_windows_package(app_id: &str, pkg_path: &Path) -> Result<String, String> {
    let settings = crate::settings::AppSettings::load();
    let configured_dir = settings.apps.install_directory.trim();
    let target_dir = if !configured_dir.is_empty() {
        PathBuf::from(configured_dir).join(app_id)
    } else {
        let local_app_data = dirs::data_local_dir().ok_or("Cannot locate LocalAppData")?;
        local_app_data.join("Programs").join("OpenCloud").join(app_id)
    };
    std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

    let file_str = pkg_path.to_string_lossy();
    if file_str.ends_with(".zip") {
        let powershell_cmd = format!(
            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            file_str,
            target_dir.to_string_lossy()
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-Command", &powershell_cmd])
            .status()
            .map_err(|e| format!("PowerShell extract failed: {}", e))?;
        if !status.success() {
            return Err("Extracting zip failed".into());
        }
        let exe = target_dir.join(format!("{}.exe", app_id));
        Ok(exe.to_string_lossy().to_string())
    } else if file_str.ends_with(".msi") {
        let status = Command::new("msiexec.exe")
            .args(["/i", &file_str, "/qn", "/norestart"])
            .status()
            .map_err(|e| format!("msiexec failed: {}", e))?;
        if !status.success() {
            return Err("MSI installer failed".into());
        }
        Ok(format!("Installed {}", app_id))
    } else {
        Err("Unsupported package format".into())
    }
}

#[cfg(target_os = "linux")]
fn install_linux_package(app_id: &str, pkg_path: &Path) -> Result<String, String> {
    let settings = crate::settings::AppSettings::load();
    let configured_dir = settings.apps.install_directory.trim();
    let (bin_dir, desktop_dir) = if !configured_dir.is_empty() {
        let p = PathBuf::from(configured_dir);
        let _ = std::fs::create_dir_all(&p);
        let desk = dirs::home_dir().map(|h| h.join(".local/share/applications")).unwrap_or_else(|| p.clone());
        (p, desk)
    } else {
        let home = dirs::home_dir().ok_or("Cannot locate home directory")?;
        (home.join(".local/bin"), home.join(".local/share/applications"))
    };

    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&desktop_dir).map_err(|e| e.to_string())?;

    let dest_bin = bin_dir.join(app_id);
    std::fs::copy(pkg_path, &dest_bin).map_err(|e| e.to_string())?;

    let _ = Command::new("chmod").args(["+x", dest_bin.to_str().unwrap()]).status();

    let desktop_content = format!(
        "[Desktop Entry]\nName={}\nExec={} %U\nTerminal=false\nType=Application\nCategories=Graphics;Office;AudioVideo;Development;\n",
        app_id,
        dest_bin.to_string_lossy()
    );
    let desktop_file = desktop_dir.join(format!("{}.desktop", app_id));
    let _ = std::fs::write(desktop_file, desktop_content);

    Ok(dest_bin.to_string_lossy().to_string())
}

pub fn uninstall_app(app_id: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let candidates = crate::scanner::detect_installed_app(app_id);
        if let Some(path) = candidates.executable_path {
            let p = PathBuf::from(path);
            if p.exists() {
                if p.is_dir() {
                    let _ = std::fs::remove_dir_all(&p);
                } else {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }

        // Also check configured directory directly
        let settings = crate::settings::AppSettings::load();
        let configured = settings.apps.install_directory.trim();
        let mut check_dirs = Vec::new();
        if !configured.is_empty() {
            check_dirs.push(PathBuf::from(configured));
        }
        check_dirs.push(PathBuf::from("/Applications"));
        if let Some(home) = dirs::home_dir() {
            check_dirs.push(home.join("Applications"));
        }

        let mut names = vec![format!("{}.app", app_id), app_id.to_string()];
        let mut chars = app_id.chars();
        if let Some(c) = chars.next() {
            names.push(format!("{}.app", c.to_uppercase().collect::<String>() + chars.as_str()));
            names.push(c.to_uppercase().collect::<String>() + chars.as_str());
        }
        for cand in crate::scanner::get_macos_app_names(app_id) {
            names.push(format!("{}.app", cand));
            names.push(cand);
        }
        names.sort();
        names.dedup();

        for dir in check_dirs {
            for name in &names {
                let p = dir.join(name);
                if p.exists() {
                    if p.is_dir() {
                        let _ = std::fs::remove_dir_all(&p);
                    } else {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            }
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        let settings = crate::settings::AppSettings::load();
        let configured = settings.apps.install_directory.trim();
        if !configured.is_empty() {
            let p = PathBuf::from(configured).join(app_id);
            if p.exists() {
                let _ = std::fs::remove_dir_all(&p);
            }
        }
        if let Some(local_app_data) = dirs::data_local_dir() {
            let app_dir = local_app_data.join("Programs").join("OpenCloud").join(app_id);
            if app_dir.exists() {
                let _ = std::fs::remove_dir_all(&app_dir);
            }
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let settings = crate::settings::AppSettings::load();
        let configured = settings.apps.install_directory.trim();
        if !configured.is_empty() {
            let p = PathBuf::from(configured).join(app_id);
            if p.exists() {
                let _ = std::fs::remove_file(&p);
            }
        }
        if let Some(home) = dirs::home_dir() {
            let bin = home.join(".local/bin").join(app_id);
            let desktop = home.join(".local/share/applications").join(format!("{}.desktop", app_id));
            let _ = std::fs::remove_file(bin);
            let _ = std::fs::remove_file(desktop);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::detect_installed_app;
    use crate::settings::AppSettings;

    #[test]
    fn test_install_uninstall_and_custom_path_lifecycle() {
        let original_settings = AppSettings::load();

        // 1. Reset all settings to defaults
        let default_settings = AppSettings::default();
        let _ = default_settings.save();

        let test_dmg = PathBuf::from("/tmp/opencloud_test_photocraft.dmg");
        let custom_dir = std::env::temp_dir().join("opencloud_lifecycle_test_dir");
        let _ = std::fs::remove_dir_all(&custom_dir);
        let _ = std::fs::create_dir_all(&custom_dir);

        if test_dmg.exists() {
            // STEP A: Install to default path
            let install_res = install_package("photocraft", &test_dmg);
            assert!(install_res.is_ok(), "Default install failed: {:?}", install_res);

            // Verify detection
            let detected = detect_installed_app("photocraft");
            assert!(detected.is_installed, "App was not detected after default install");

            // STEP B: Uninstall from default path
            let uninst_res = uninstall_app("photocraft");
            assert!(uninst_res.is_ok(), "Uninstall failed: {:?}", uninst_res);
            let detected_after_uninst = detect_installed_app("photocraft");
            assert!(!detected_after_uninst.is_installed, "App still detected after uninstall");

            // STEP C: Change default install path to custom path
            let mut custom_settings = AppSettings::default();
            custom_settings.apps.install_directory = custom_dir.to_string_lossy().to_string();
            assert!(custom_settings.save().is_ok());

            // STEP D: Reinstall to custom path
            let custom_install_res = install_package("photocraft", &test_dmg);
            assert!(custom_install_res.is_ok(), "Custom path install failed: {:?}", custom_install_res);

            // Verify app exists inside custom directory
            let installed_in_custom = custom_dir.join("PhotoCraft.app");
            assert!(installed_in_custom.exists(), "PhotoCraft.app missing from custom dir");
            let detected_custom = detect_installed_app("photocraft");
            assert!(detected_custom.is_installed, "App was not detected in custom directory");

            // STEP E: Delete / uninstall from custom directory
            let uninst_custom = uninstall_app("photocraft");
            assert!(uninst_custom.is_ok(), "Uninstall from custom dir failed");
            assert!(!installed_in_custom.exists(), "PhotoCraft.app still in custom dir after deletion");
            let detected_deleted = detect_installed_app("photocraft");
            assert!(!detected_deleted.is_installed, "App still detected after deletion from custom dir");

            // STEP F: Reset all settings back to default
            let reset_settings = AppSettings::default();
            assert!(reset_settings.save().is_ok());
            let reloaded = AppSettings::load();
            assert_eq!(reloaded.apps.install_directory, AppSettings::default().apps.install_directory, "Settings not reset to default");
        } else {
            // Mock bundle test if DMG not cached
            let mock_pkg = std::env::temp_dir().join("TestMockApp.app");
            let _ = std::fs::create_dir_all(mock_pkg.join("Contents/MacOS"));
            let _ = std::fs::write(mock_pkg.join("Contents/MacOS/TestMockApp"), b"#!/bin/sh\nexit 0\n");

            // Test custom path install with mock bundle
            let mut custom_settings = AppSettings::default();
            custom_settings.apps.install_directory = custom_dir.to_string_lossy().to_string();
            assert!(custom_settings.save().is_ok());

            let res = install_package("photocraft", &mock_pkg);
            assert!(res.is_ok());
            assert!(custom_dir.join("TestMockApp.app").exists());

            let _ = uninstall_app("photocraft");
            let _ = std::fs::remove_dir_all(&mock_pkg);
            let _ = std::fs::remove_dir_all(&custom_dir);

            let reset_settings = AppSettings::default();
            assert!(reset_settings.save().is_ok());
        }

        // Cleanup temp test directory
        let _ = std::fs::remove_dir_all(&custom_dir);

        // Restore original user settings
        let original = original_settings;
        let _ = original.save();
    }
}
