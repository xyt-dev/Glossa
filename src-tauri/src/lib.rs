mod commands;
mod state;

use state::AppState;

/// WebKitGTK honors http(s)_proxy even for localhost; without a no_proxy
/// exemption the vite dev server gets routed through the system proxy and the
/// window renders blank. Only relevant in dev (release serves the bundled
/// frontend over a custom scheme).
#[cfg(all(debug_assertions, target_os = "linux"))]
fn exempt_localhost_from_proxy() {
    let cur = std::env::var("no_proxy")
        .or_else(|_| std::env::var("NO_PROXY"))
        .unwrap_or_default();
    let mut parts: Vec<String> =
        cur.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
    for host in ["localhost", "127.0.0.1"] {
        if !parts.iter().any(|p| p == host) {
            parts.push(host.to_string());
        }
    }
    let merged = parts.join(",");
    std::env::set_var("no_proxy", &merged);
    std::env::set_var("NO_PROXY", &merged);
}

pub fn run() {
    #[cfg(all(debug_assertions, target_os = "linux"))]
    exempt_localhost_from_proxy();

    let state = AppState::init().expect("failed to initialize glossa kernel");
    let initial_zoom = state.config.try_lock().map(|c| c.ui.zoom).unwrap_or(1.0);
    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            use tauri::Manager;
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.set_zoom(initial_zoom);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_sessions,
            commands::create_session,
            commands::load_session,
            commands::delete_session,
            commands::rename_session,
            commands::send_message,
            commands::mark_word,
            commands::unmark_word,
            commands::get_memory,
            commands::get_config,
            commands::set_config,
            commands::set_zoom,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
