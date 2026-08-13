pub mod cli;
mod commands;
mod guard;
mod history;
mod http;
mod importer;
mod models;
mod oauth;
mod redaction;
mod response_tools;
mod runner;
mod secrets;
mod security;
mod session;
mod stream;
mod variables;
mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(stream::StreamState::default())
        .manage(session::SessionState::default())
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
            commands::runner::run_collection,
            commands::secrets::list_secrets,
            commands::secrets::save_secret,
            commands::secrets::delete_secret,
            commands::security::inspect_request,
            commands::security::generate_safe_curl,
            commands::stream::connect_stream,
            commands::stream::send_stream_message,
            commands::stream::disconnect_stream,
            commands::session::list_cookies,
            commands::session::delete_cookie,
            commands::session::clear_cookies,
            commands::session::close_workspace_session,
            commands::response_tools::export_response,
            commands::response_tools::save_response_fixture,
        ])
        .run(tauri::generate_context!())
        .expect("не удалось запустить ReqVault");
}
