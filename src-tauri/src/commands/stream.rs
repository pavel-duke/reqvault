use tauri::{State, ipc::Channel};

use crate::{
    models::{StreamConnectConfig, StreamEvent},
    stream::{self, StreamState},
};

#[tauri::command]
pub fn connect_stream(
    state: State<'_, StreamState>,
    config: StreamConnectConfig,
    events: Channel<StreamEvent>,
) -> Result<String, String> {
    stream::connect(&state, config, events)
}

#[tauri::command]
pub fn send_stream_message(
    state: State<'_, StreamState>,
    session_id: String,
    message: String,
) -> Result<(), String> {
    stream::send(&state, &session_id, message)
}

#[tauri::command]
pub fn disconnect_stream(state: State<'_, StreamState>, session_id: String) -> Result<(), String> {
    stream::disconnect(&state, &session_id)
}
