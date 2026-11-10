mod builtin;
mod commands;
mod job;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::CancelFlag::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_tools,
            commands::run_job,
            commands::cancel_job
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
