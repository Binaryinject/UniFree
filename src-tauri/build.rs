fn main() {
    let mut attributes = tauri_build::Attributes::new();

    if std::env::var("PROFILE").as_deref() == Ok("release") {
        let windows = tauri_build::WindowsAttributes::new()
            .app_manifest(include_str!("app.manifest"));
        attributes = attributes.windows_attributes(windows);
    }

    tauri_build::try_build(attributes)
        .expect("failed to run tauri build script");
}
