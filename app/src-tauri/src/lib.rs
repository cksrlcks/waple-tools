mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Jobs::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_tools,
            commands::new_job_id,
            commands::preview_job,
            commands::run_job,
            commands::cancel_job,
            commands::undo_job
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
