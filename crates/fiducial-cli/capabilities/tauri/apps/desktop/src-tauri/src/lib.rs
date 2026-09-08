use fiducial_tauri::{list_ports, PortInfo, SerialTransport};
use std::sync::Mutex;
use tauri::State;

struct AppState {
    transport: Mutex<Option<SerialTransport>>,
}

#[tauri::command]
fn cmd_list_ports() -> Result<Vec<PortInfo>, String> {
    list_ports().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_connect(
    state: State<'_, AppState>,
    path: String,
    baud_rate: u32,
) -> Result<(), String> {
    let transport = SerialTransport::open(&path, baud_rate).map_err(|e| e.to_string())?;
    *state.transport.lock().unwrap() = Some(transport);
    Ok(())
}

#[tauri::command]
fn cmd_disconnect(state: State<'_, AppState>) {
    *state.transport.lock().unwrap() = None;
}

#[tauri::command]
fn cmd_send(state: State<'_, AppState>, payload: Vec<u8>) -> Result<(), String> {
    let mut guard = state.transport.lock().unwrap();
    match guard.as_mut() {
        Some(t) => t.send(&payload).map_err(|e| e.to_string()),
        None => Err("not connected".into()),
    }
}

#[tauri::command]
fn cmd_recv(state: State<'_, AppState>) -> Result<Vec<u8>, String> {
    let mut guard = state.transport.lock().unwrap();
    match guard.as_mut() {
        Some(t) => t.recv().map_err(|e| e.to_string()),
        None => Err("not connected".into()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            transport: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            cmd_list_ports,
            cmd_connect,
            cmd_disconnect,
            cmd_send,
            cmd_recv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
