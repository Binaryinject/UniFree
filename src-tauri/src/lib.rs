mod alf_generator;
mod app_config;
mod commands;
mod config_patcher;
mod license;
mod patcher;
mod scanner;
mod ulf_signer;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::scan_unity_editors,
            commands::check_hub_dll_status,
            commands::check_hub_config_status,
            commands::patch_editor_dll,
            commands::patch_hub,
            commands::restore_dll,
            commands::restore_hub,
            commands::copy_license,
            commands::check_license_status,
            commands::check_admin,
            commands::relaunch_as_admin,
            commands::check_process,
            commands::kill_process,
            commands::launch_hub,
            commands::open_browser,
            commands::generate_alf,
            commands::generate_license_direct,
            commands::get_hub_path,
            commands::select_hub_path,
            commands::reset_hub_path,
            commands::get_editor_scan_paths,
            commands::add_editor_scan_path,
            commands::remove_editor_scan_path,

        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
