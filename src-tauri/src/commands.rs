use futures_util::StreamExt;
use kernel::agent::SendEvent;
use kernel::app::AppCore;
use kernel::config::Config;
use kernel::memory::{MarkInput, MarkKind, VocabEntry, VocabMemory};
use kernel::store::{Session, SessionMeta};
use kernel::Mode;
use tauri::ipc::Channel;
use tauri::State;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
pub async fn list_sessions(core: State<'_, std::sync::Arc<AppCore>>) -> CmdResult<Vec<SessionMeta>> {
    core.list_sessions().await.map_err(err)
}

#[tauri::command]
pub async fn create_session(core: State<'_, std::sync::Arc<AppCore>>) -> CmdResult<Session> {
    core.create_session().await.map_err(err)
}

#[tauri::command]
pub async fn load_session(core: State<'_, std::sync::Arc<AppCore>>, id: String) -> CmdResult<Session> {
    core.load_session(&id).await.map_err(err)
}

#[tauri::command]
pub async fn delete_session(core: State<'_, std::sync::Arc<AppCore>>, id: String) -> CmdResult<()> {
    core.delete_session(&id).await.map_err(err)
}

#[tauri::command]
pub async fn rename_session(
    core: State<'_, std::sync::Arc<AppCore>>,
    id: String,
    title: String,
) -> CmdResult<Session> {
    core.rename_session(&id, &title).await.map_err(err)
}

/// One agent turn; streams SendEvent to the frontend over `on_event`.
/// The invoke promise resolves once the turn is fully persisted.
#[tauri::command]
pub async fn send_message(
    core: State<'_, std::sync::Arc<AppCore>>,
    session_id: String,
    text: String,
    mode: Mode,
    on_event: Channel<SendEvent>,
) -> CmdResult<()> {
    let mut stream = core.send_message(session_id, text, mode).await.map_err(err)?;
    while let Some(ev) = stream.next().await {
        // frontend gone — nothing left to notify, agent still persists the turn
        if on_event.send(ev).is_err() {
            break;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn mark_word(core: State<'_, std::sync::Arc<AppCore>>, input: MarkInput) -> CmdResult<VocabEntry> {
    core.mark(input).await.map_err(err)
}

#[tauri::command]
pub async fn unmark_word(
    core: State<'_, std::sync::Arc<AppCore>>,
    word: String,
    kind: MarkKind,
) -> CmdResult<()> {
    core.unmark(&word, kind).await.map_err(err)
}

#[tauri::command]
pub async fn get_memory(core: State<'_, std::sync::Arc<AppCore>>) -> CmdResult<VocabMemory> {
    core.memory().await.map_err(err)
}

#[tauri::command]
pub async fn get_config(core: State<'_, std::sync::Arc<AppCore>>) -> CmdResult<Config> {
    Ok(core.config().await)
}

#[tauri::command]
pub async fn set_config(
    core: State<'_, std::sync::Arc<AppCore>>,
    web: State<'_, crate::web::WebServer>,
    config: Config,
) -> CmdResult<()> {
    core.set_config(config.clone()).await.map_err(err)?;
    // 内嵌 Web 服务随配置启停/换端口，bind 失败直接报给设置界面
    web.sync(&config, core.inner().clone()).await
}

#[tauri::command]
pub async fn set_zoom(window: tauri::WebviewWindow, zoom: f64) -> CmdResult<()> {
    window.set_zoom(zoom.clamp(0.5, 3.0)).map_err(err)
}
