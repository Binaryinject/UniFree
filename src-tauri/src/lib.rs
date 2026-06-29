mod commands;
mod config;
mod config_patcher;
mod i18n;
mod il_patcher;
mod license;
mod patcher;
mod scanner;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_system_lang,
            commands::scan_unity_editors,
            commands::check_editor_dll_status,
            commands::check_hub_dll_status,
            commands::check_hub_config_status,
            commands::check_hub_cert_status,
            commands::patch_editor_dll,
            commands::patch_hub,
            commands::restore_dll,
            commands::restore_hub,
            commands::write_sign_in_config,
            commands::delete_sign_in_config,
            commands::copy_license,
            commands::check_license_status,
            commands::check_admin,
            commands::relaunch_as_admin,
            commands::get_hub_dll_path,
            commands::check_process,
            commands::kill_process,
            commands::launch_hub,
            commands::patch_dll_il,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
