//! 桌面端内嵌 Web 服务的生命周期管理（随配置 [web] enabled/port 启停）。

use std::sync::{Arc, Mutex};

use kernel::app::AppCore;
use kernel::config::Config;

#[derive(Clone, Default)]
pub struct WebServer {
    /// (运行中的端口, 任务句柄)
    handle: Arc<Mutex<Option<(u16, tauri::async_runtime::JoinHandle<()>)>>>,
}

impl WebServer {
    /// 让运行状态与配置一致：按需启动/停止/换端口重启。
    /// bind 在这里同步完成，端口冲突等错误能直接反馈给设置界面。
    pub async fn sync(&self, config: &Config, core: Arc<AppCore>) -> Result<(), String> {
        {
            let mut slot = self.handle.lock().unwrap();
            if let Some((port, _)) = slot.as_ref() {
                if config.web.enabled && *port == config.web.port {
                    return Ok(()); // 已按当前配置运行
                }
            }
            if let Some((port, task)) = slot.take() {
                task.abort();
                println!("Web 服务已关闭（端口 {port}）");
            }
            if !config.web.enabled {
                return Ok(());
            }
        }

        let port = config.web.port;
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(|e| format!("Web 端口 {port} 绑定失败：{e}"))?;
        let token = std::env::var("GLOSSA_TOKEN").ok().filter(|t| !t.is_empty());
        glossa_server::print_started("0.0.0.0".parse().unwrap(), port);

        let task = tauri::async_runtime::spawn(async move {
            if let Err(e) = glossa_server::serve(listener, token, core).await {
                eprintln!("内嵌 Web 服务错误: {e}");
            }
        });
        *self.handle.lock().unwrap() = Some((port, task));
        Ok(())
    }
}
