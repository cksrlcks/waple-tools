use std::time::Duration;

mod commands;

/// 보관 기간이 지난 작업 기록을 지운다 (0이면 정리 안 함)
fn prune_old_logs() {
    let days =
        localtools_builtin::load_settings(&localtools_builtin::settings_path()).log_retention_days;
    if days > 0 {
        let age = Duration::from_secs(u64::from(days) * 86_400);
        localtools_core::batch::prune_logs(&localtools_builtin::default_log_dir(), age);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    prune_old_logs();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Jobs::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_tools,
            commands::new_job_id,
            commands::preview_job,
            commands::run_job,
            commands::cancel_job,
            commands::undo_job,
            commands::get_settings,
            commands::set_settings,
            commands::stat_paths
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
