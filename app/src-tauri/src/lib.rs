use std::time::Duration;

mod addons;
mod commands;
mod tray;
mod view_limit;

/// 보관 기간이 지난 작업 기록을 지운다 (0이면 정리 안 함)
fn prune_old_logs() {
    let days = waple_tools_builtin::load_settings(&waple_tools_builtin::settings_path())
        .log_retention_days;
    if days > 0 {
        let age = Duration::from_secs(u64::from(days) * 86_400);
        waple_tools_core::batch::prune_logs(&waple_tools_builtin::default_log_dir(), age);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    prune_old_logs();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // 앱 자동 업데이트: GitHub Releases의 latest.json만 확인, 서명이 맞아야 설치
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(commands::Jobs::default())
        .manage(addons::Addons::from_env())
        .on_window_event(tray::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::list_tools,
            commands::install_tool,
            commands::uninstall_tool,
            commands::refresh_addons,
            commands::new_job_id,
            commands::preview_job,
            commands::run_job,
            commands::cancel_job,
            commands::undo_job,
            commands::get_settings,
            commands::set_settings,
            commands::stat_paths,
            commands::thumbnail,
            commands::preview_tool,
            commands::run_interactive,
            commands::quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
