mod commands;
mod guard;
mod history;
mod http;
mod importer;
mod models;
mod oauth;
mod redaction;
mod secrets;
mod security;
mod variables;
mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace::create_workspace,
            commands::workspace::open_workspace,
            commands::workspace::save_workspace_config,
            commands::workspace::export_workspace,
            commands::workspace::import_workspace,
            commands::workspace::save_request,
            commands::workspace::delete_request,
            commands::workspace::save_environment,
            commands::workspace::delete_environment,
            commands::http::send_request,
            commands::history::get_history_settings,
            commands::history::set_history_settings,
            commands::history::list_history,
            commands::history::get_history_entry,
            commands::history::delete_history_entry,
            commands::history::clear_history,
            commands::importer::import_collection,
            commands::importer::import_curl,
            commands::oauth::authorize_oauth,
            commands::oauth::refresh_oauth,
            commands::secrets::list_secrets,
            commands::secrets::save_secret,
            commands::secrets::delete_secret,
            commands::security::inspect_request,
            commands::security::generate_safe_curl,
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить ReqVault");
}
