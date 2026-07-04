//! Glossa web server：kernel::AppCore 的 HTTP 适配层（lib，供独立 bin 与桌面端内嵌共用）。
//! REST + NDJSON 流式消息，同源静态服务内嵌的前端（ui/dist）。
//!
//! 默认绑 0.0.0.0:8040（局域网可直接访问）。设置 GLOSSA_TOKEN 后 /api 要求
//! `Authorization: Bearer <token>`（前端用 `?token=` 传入后自动携带）；
//! 未设 token 而绑非回环地址时打印告警（config 接口包含 API key）。

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Path, Request, State};
use axum::http::{header, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use kernel::app::AppCore;
use kernel::config::Config;
use kernel::memory::{MarkInput, MarkKind};
use kernel::Mode;
use rust_embed::RustEmbed;
use serde::Deserialize;

#[derive(RustEmbed)]
#[folder = "../../ui/dist"]
struct WebAssets;

#[derive(Clone)]
struct Ctx {
    core: Arc<AppCore>,
    token: Option<Arc<String>>,
}

type ApiError = (StatusCode, String);
type ApiResult<T> = Result<T, ApiError>;

fn err(e: kernel::Error) -> ApiError {
    let status = match &e {
        kernel::Error::SessionNotFound(_) | kernel::Error::ProfileNotFound(_) => {
            StatusCode::NOT_FOUND
        }
        kernel::Error::MissingApiKey(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, e.to_string())
}

/// 独立 bin 与 `glossa web` 子命令共用的入口（自建 tokio runtime）。
pub fn run_blocking(args: Vec<String>) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(run_cli(args));
}

pub async fn run_cli(args: Vec<String>) {
    let mut host: IpAddr = "0.0.0.0".parse().unwrap();
    let mut port: u16 = 8040;
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--host" | "-H" => {
                let v = args.next().expect("--host 需要参数");
                host = v.parse().expect("非法的 host 地址");
            }
            "--port" | "-p" => {
                let v = args.next().expect("--port 需要参数");
                port = v.parse().expect("非法的端口");
            }
            "--help" | "-h" => {
                println!(
                    "glossa web [--port 8040] [--host 0.0.0.0]\n\
                     默认允许局域网访问；设置 GLOSSA_TOKEN 后 /api 需携带 token。"
                );
                return;
            }
            other => {
                eprintln!("未知参数: {other}（--help 查看用法）");
                std::process::exit(2);
            }
        }
    }

    let token = std::env::var("GLOSSA_TOKEN").ok().filter(|t| !t.is_empty());
    if !host.is_loopback() && token.is_none() {
        eprintln!(
            "警告：Web 服务对局域网开放且未设置鉴权（配置接口包含 API key）。\n\
             如需鉴权请设置 GLOSSA_TOKEN，浏览器用 ?token=<token> 访问。"
        );
    }

    let core = Arc::new(AppCore::init().expect("初始化 glossa kernel 失败"));
    let addr = SocketAddr::new(host, port);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("端口绑定失败");
    print_started(host, port);
    if let Err(e) = serve(listener, token, core).await {
        eprintln!("server error: {e}");
    }
}

/// 真实局域网 IP：枚举网卡，跳过虚拟设备（TUN 代理、docker 网桥等——
/// 否则 Clash TUN 的 198.18.x fake-ip 会被当成局域网地址），优先私网 IPv4。
fn lan_ip() -> Option<IpAddr> {
    const VIRTUAL_PREFIXES: [&str; 8] =
        ["tun", "tap", "docker", "br-", "veth", "virbr", "wg", "zt"];
    let mut candidates: Vec<IpAddr> = if_addrs::get_if_addrs()
        .ok()?
        .into_iter()
        .filter(|iface| {
            !iface.is_loopback()
                && !VIRTUAL_PREFIXES.iter().any(|p| iface.name.starts_with(p))
        })
        .filter_map(|iface| match iface.ip() {
            IpAddr::V4(v4) if !v4.is_link_local() => Some(IpAddr::V4(v4)),
            _ => None,
        })
        .collect();
    candidates.sort_by_key(|ip| match ip {
        IpAddr::V4(v4) if v4.is_private() => 0,
        _ => 1,
    });
    candidates.first().copied()
}

/// 可访问的入口链接：不展示 0.0.0.0，展示实际可点开的地址。
pub fn access_urls(host: IpAddr, port: u16) -> Vec<(&'static str, String)> {
    if host.is_loopback() {
        vec![("本机", format!("http://127.0.0.1:{port}/"))]
    } else if host.is_unspecified() {
        let mut urls = vec![("本机", format!("http://127.0.0.1:{port}/"))];
        if let Some(ip) = lan_ip() {
            urls.push(("局域网", format!("http://{ip}:{port}/")));
        }
        urls
    } else {
        vec![("外部", format!("http://{host}:{port}/"))]
    }
}

pub fn print_started(host: IpAddr, port: u16) {
    println!("Web 服务已开启：");
    for (label, url) in access_urls(host, port) {
        // 标签是 CJK（每字 2 列），按显示宽度补齐到 8 列再接 URL
        let pad = 8usize.saturating_sub(label.chars().count() * 2);
        println!("  {label}{}{url}", " ".repeat(pad));
    }
}

/// 在已绑定的 listener 上运行服务（桌面端内嵌复用；bind 由调用方负责，便于把
/// 端口冲突等错误直接反馈给 UI）。
pub async fn serve(
    listener: tokio::net::TcpListener,
    token: Option<String>,
    core: Arc<AppCore>,
) -> std::io::Result<()> {
    axum::serve(listener, router(core, token)).await
}

pub fn router(core: Arc<AppCore>, token: Option<String>) -> Router {
    let ctx = Ctx { core, token: token.map(Arc::new) };

    let api = Router::new()
        .route("/config", get(get_config).put(set_config))
        .route("/sessions", get(list_sessions).post(create_session))
        .route(
            "/sessions/{id}",
            get(load_session).delete(delete_session).patch(rename_session),
        )
        .route("/sessions/{id}/messages", post(send_message))
        .route("/memory", get(get_memory))
        .route("/memory/mark", post(mark_word))
        .route("/memory/unmark", post(unmark_word))
        .layer(middleware::from_fn_with_state(ctx.clone(), auth));

    Router::new()
        .nest("/api", api)
        .fallback(static_assets)
        .with_state(ctx)
}

/// /api 鉴权：设置了 GLOSSA_TOKEN 时要求 Bearer token。
async fn auth(State(ctx): State<Ctx>, req: Request, next: Next) -> Response {
    if let Some(expected) = &ctx.token {
        let ok = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| t == expected.as_str());
        if !ok {
            return (StatusCode::UNAUTHORIZED, "invalid or missing token").into_response();
        }
    }
    next.run(req).await
}

async fn get_config(State(ctx): State<Ctx>) -> Json<Config> {
    Json(ctx.core.config().await)
}

async fn set_config(State(ctx): State<Ctx>, Json(cfg): Json<Config>) -> ApiResult<()> {
    ctx.core.set_config(cfg).await.map_err(err)
}

async fn list_sessions(State(ctx): State<Ctx>) -> ApiResult<impl IntoResponse> {
    ctx.core.list_sessions().await.map(Json).map_err(err)
}

async fn create_session(State(ctx): State<Ctx>) -> ApiResult<impl IntoResponse> {
    ctx.core.create_session().await.map(Json).map_err(err)
}

async fn load_session(
    State(ctx): State<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    ctx.core.load_session(&id).await.map(Json).map_err(err)
}

async fn delete_session(State(ctx): State<Ctx>, Path(id): Path<String>) -> ApiResult<()> {
    ctx.core.delete_session(&id).await.map_err(err)
}

#[derive(Deserialize)]
struct RenameBody {
    title: String,
}

async fn rename_session(
    State(ctx): State<Ctx>,
    Path(id): Path<String>,
    Json(body): Json<RenameBody>,
) -> ApiResult<impl IntoResponse> {
    ctx.core.rename_session(&id, &body.title).await.map(Json).map_err(err)
}

#[derive(Deserialize)]
struct SendBody {
    text: String,
    mode: Mode,
}

/// 一轮 agent 回合：响应体为 NDJSON 流，每行一个 SendEvent。
async fn send_message(
    State(ctx): State<Ctx>,
    Path(id): Path<String>,
    Json(body): Json<SendBody>,
) -> ApiResult<Response> {
    let stream = ctx.core.send_message(id, body.text, body.mode).await.map_err(err)?;
    let ndjson = stream.map(|ev| {
        let mut line = serde_json::to_vec(&ev).unwrap_or_default();
        line.push(b'\n');
        Ok::<_, std::convert::Infallible>(Bytes::from(line))
    });
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .header(header::CACHE_CONTROL, "no-cache")
        // 反向代理（nginx 等）常会缓冲流式响应，显式关闭
        .header("x-accel-buffering", "no")
        .body(Body::from_stream(ndjson))
        .unwrap())
}

async fn get_memory(State(ctx): State<Ctx>) -> ApiResult<impl IntoResponse> {
    ctx.core.memory().await.map(Json).map_err(err)
}

async fn mark_word(
    State(ctx): State<Ctx>,
    Json(input): Json<MarkInput>,
) -> ApiResult<impl IntoResponse> {
    ctx.core.mark(input).await.map(Json).map_err(err)
}

#[derive(Deserialize)]
struct UnmarkBody {
    word: String,
    kind: MarkKind,
}

async fn unmark_word(State(ctx): State<Ctx>, Json(body): Json<UnmarkBody>) -> ApiResult<()> {
    ctx.core.unmark(&body.word, body.kind).await.map_err(err)
}

/// 内嵌前端静态资源；未知路径回退 index.html（保持刷新可用）。
async fn static_assets(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match WebAssets::get(path).or_else(|| WebAssets::get("index.html")) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [
                    (header::CONTENT_TYPE, mime.as_ref().to_string()),
                    // hashed assets can be cached hard; index.html must not
                    (
                        header::CACHE_CONTROL,
                        if path.starts_with("assets/") {
                            "public, max-age=31536000, immutable".to_string()
                        } else {
                            "no-cache".to_string()
                        },
                    ),
                ],
                file.data,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
