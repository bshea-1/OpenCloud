fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rerun-if-changed=src/platform/macos.m");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        cc::Build::new()
            .file("src/platform/macos.m")
            .compile("opencloud_macos");
    }
}
