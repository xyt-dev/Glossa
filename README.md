# Glossa

> γλῶσσα — 语言；gloss — 生词旁的注解。

翻译 + 英语学习的桌面应用：严格翻译之外，模型会基于你的**生词 memory** 挑选 IELTS 7+ 词汇做
结构化讲解（音标 / 词性 / native 用法 / 例句对照），并通过「标记生词 / 标记用法」持续画像你的
英语水平，让讲解深度越来越贴合。

- **kernel / UI 分离**：`crates/kernel` 是纯 Rust 核心库（可接任意前端），`src-tauri` + `ui/`（React 19）是 Tauri 2 桌面壳。
- **会话式 agent UI**：主区是一条对话流；严格翻译与聊天共用一个输入框（Ctrl+M 切换，默认严格翻译）。
  翻译回合强制结构化 JSON 输出（`response_format` + 解析失败自动修复重试 + 降级），聊天回合流式 markdown，
  天然携带本会话所有翻译作为上下文。
- **任意 OpenAI 兼容 API**：TOML 多 profile（base_url / api_key / model / reasoning_effort / temperature / 任意额外字段透传），默认 DeepSeek。
- **主题**：Gruvbox (morhetz) 深/浅 + Catppuccin Mocha/Latte 官方调色板，界面缩放可调，设置内即时预览。

## 安装

从 [Releases](../../releases) 下载对应平台的包（由 CI 三平台自动构建）：

### Linux

| 包 | 安装方式 |
|---|---|
| `Glossa_x.y.z_amd64.AppImage` | `chmod +x` 后直接运行，WebKitGTK 已内置，任意发行版可用 |
| `Glossa_x.y.z_amd64.deb` | `sudo apt install ./Glossa_x.y.z_amd64.deb`（自动安装 webkit2gtk-4.1 依赖） |
| `Glossa-x.y.z-1.x86_64.rpm` | `sudo dnf install ./Glossa-x.y.z-1.x86_64.rpm` |

Arch 用户建议从源码构建（见下），运行时依赖仅 `webkit2gtk-4.1`。

### Windows

下载 `Glossa_x.y.z_x64-setup.exe` 或 `.msi` 双击安装。需要 **WebView2 Runtime**：
Windows 11 自带；Windows 10 上安装器会自动引导安装。
未签名应用首次运行若被 SmartScreen 拦截：点「更多信息」→「仍要运行」。

### macOS

下载 `Glossa_x.y.z_aarch64.dmg`（Apple Silicon），拖入 Applications。
未签名应用首次打开：右键 →「打开」，或执行 `xattr -cr /Applications/Glossa.app` 后再启动。

### 从源码构建（全平台）

前置：**Rust stable** + **Node ≥ 20**，另加各平台 webview 依赖：

```bash
# Arch
sudo pacman -S webkit2gtk-4.1 base-devel
# Debian / Ubuntu
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev librsvg2-dev
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel
# Windows：安装 Visual Studio Build Tools（C++ 工作负载）；WebView2 参见上文
# macOS：xcode-select --install
```

```bash
cargo install tauri-cli --locked   # 首次
npm --prefix ui install            # 首次
cargo tauri build                  # 产物在 target/release/bundle/
```

### 从源码安装为系统命令（Linux）

想把 Glossa 作为 `glossa` 命令安装到系统里，可以在仓库根目录执行：

```bash
# 1) 安装 Linux 运行/构建依赖（按发行版三选一）
sudo pacman -S webkit2gtk-4.1 base-devel
# sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev librsvg2-dev
# sudo dnf install webkit2gtk4.1-devel openssl-devel

# 2) 构建前端静态资源，供 Tauri 二进制嵌入
npm --prefix ui ci
npm --prefix ui run build

# 3) 安装 GUI 启动命令到 Cargo bin 目录。注意 path 是 src-tauri，不是仓库根目录。
cargo install --path src-tauri --locked --features custom-protocol --force
glossa
```

`cargo install` 默认把二进制放到 `~/.cargo/bin/glossa`。如果终端找不到 `glossa`，
先确认 `~/.cargo/bin` 已加入 `PATH`：

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.profile
source ~/.profile
```

如果希望放到全局 `/usr/local/bin`，也可以在 `cargo install` 后执行：

```bash
sudo install -Dm755 "$HOME/.cargo/bin/glossa" /usr/local/bin/glossa
```

## 开发

```bash
cargo tauri dev        # 开发运行（热更新）
cargo test -p kernel   # 核心库测试（wiremock 本地 mock，无需真实 key）
```

## 配置

首次运行生成带注释的 `~/.config/glossa/config.toml`（Windows：`%APPDATA%\glossa\`，
macOS：`~/Library/Application Support/glossa/`），也可在应用内「设置」修改：
API profiles、模型、effort、IELTS band 下限、默认模式、主题、界面缩放（`[ui] zoom`）。

`zoom` 也可以直接写在配置文件里，例如：

```toml
[ui]
theme = "gruvbox-light"
zoom = 1.0
```
API key 直接写入 `api_key`，或留空并导出环境变量（默认 `DEEPSEEK_API_KEY`）。

数据文件（生词本 `vocab.json`、会话 `sessions/*.json`）在平台数据目录
（Linux：`~/.local/share/glossa/`），纯 JSON，可手改、可同步。

## 架构

```
crates/kernel   config / client(SSE) / schema / prompt / memory / store / agent
src-tauri       Tauri 2 commands + Channel 流式转发
ui              React 19 + Vite + TS（Sidebar / Conversation / WordCard / Settings）
```
