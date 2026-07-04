// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // `glossa web`：只跑 HTTP 服务，不拉起 GUI
        Some("web") => glossa_server::run_blocking(args[1..].to_vec()),
        None | Some("app") => glossa_lib::run(),
        Some("--help") | Some("-h") => {
            println!(
                "glossa [app]                     桌面端（默认）\n\
                 glossa web [--port 8040]         Web 服务（默认 0.0.0.0:8040，局域网可访问）"
            );
        }
        Some(other) => {
            eprintln!("未知子命令: {other}（--help 查看用法）");
            std::process::exit(2);
        }
    }
}
