pub mod cli;
mod commands;
mod fs_utils;
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

#[cfg(desktop)]
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(
        |app, _arguments, _working_directory| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        },
    ));

    builder
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
            commands::workspace::move_requests,
            commands::workspace::duplicate_requests,
            commands::workspace::rename_request,
            commands::workspace::save_environment,
            commands::workspace::delete_environment,
            commands::workspace::workspace_fingerprint,
            commands::workspace::diagnose_workspace,
            commands::workspace::preview_workspace_migration,
            commands::workspace::migrate_workspace,
            commands::workspace::rollback_workspace_migration,
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
