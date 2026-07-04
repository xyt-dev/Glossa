use futures_util::StreamExt;
use kernel::agent::{self, SendEvent};
use kernel::config::Config;
use kernel::memory::{MarkInput, MarkKind, VocabEntry, VocabMemory};
use kernel::store::{Session, SessionMeta};
use kernel::Mode;
use tauri::ipc::Channel;
use tauri::State;

use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> CmdResult<Vec<SessionMeta>> {
    let config = state.config.lock().await.clone();
    AppState::session_store(&config).list().map_err(err)
}

#[tauri::command]
pub async fn create_session(state: State<'_, AppState>) -> CmdResult<Session> {
    let config = state.config.lock().await.clone();
    AppState::session_store(&config).create().map_err(err)
}

#[tauri::command]
pub async fn load_session(state: State<'_, AppState>, id: String) -> CmdResult<Session> {
    let config = state.config.lock().await.clone();
    AppState::session_store(&config).load(&id).map_err(err)
}

#[tauri::command]
pub async fn delete_session(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let config = state.config.lock().await.clone();
    AppState::session_store(&config).delete(&id).map_err(err)
}

#[tauri::command]
pub async fn rename_session(
    state: State<'_, AppState>,
    id: String,
    title: String,
) -> CmdResult<Session> {
    let config = state.config.lock().await.clone();
    AppState::session_store(&config)
        .rename(&id, &title)
        .map_err(err)
}

/// One agent turn; streams SendEvent to the frontend over `on_event`.
/// The invoke promise resolves once the turn is fully persisted.
#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    session_id: String,
    text: String,
    mode: Mode,
    on_event: Channel<SendEvent>,
) -> CmdResult<()> {
    let config = state.config.lock().await.clone();
    let memory = AppState::memory_store(&config);
    let store = AppState::session_store(&config);
    let mut stream = agent::send(
        state.client.clone(),
        config,
        memory,
        store,
        session_id,
        text,
        mode,
    )
    .await
    .map_err(err)?;
    while let Some(ev) = stream.next().await {
        // frontend gone — nothing left to notify, agent still persists the turn
        if on_event.send(ev).is_err() {
            break;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn mark_word(state: State<'_, AppState>, input: MarkInput) -> CmdResult<VocabEntry> {
    let config = state.config.lock().await.clone();
    AppState::memory_store(&config).mark(input).map_err(err)
}

#[tauri::command]
pub async fn unmark_word(
    state: State<'_, AppState>,
    word: String,
    kind: MarkKind,
) -> CmdResult<()> {
    let config = state.config.lock().await.clone();
    AppState::memory_store(&config)
        .unmark(&word, kind)
        .map_err(err)
}

#[tauri::command]
pub async fn get_memory(state: State<'_, AppState>) -> CmdResult<VocabMemory> {
    let config = state.config.lock().await.clone();
    AppState::memory_store(&config).load().map_err(err)
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> CmdResult<Config> {
    Ok(state.config.lock().await.clone())
}

#[tauri::command]
pub async fn set_config(state: State<'_, AppState>, config: Config) -> CmdResult<()> {
    config.save(&state.config_path).map_err(err)?;
    *state.config.lock().await = config;
    Ok(())
}

#[tauri::command]
pub async fn set_zoom(window: tauri::WebviewWindow, zoom: f64) -> CmdResult<()> {
    window.set_zoom(zoom.clamp(0.5, 3.0)).map_err(err)
}
