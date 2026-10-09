#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn setup_macos_app(icon_png_data: *const u8, len: usize);
}

pub fn configure_macos_metadata() {
    #[cfg(target_os = "macos")]
    unsafe {
        let icon_bytes = include_bytes!("../../assets/icons/app_icon.png");
        setup_macos_app(icon_bytes.as_ptr(), icon_bytes.len());
    }
}
