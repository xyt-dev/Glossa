# Change Log

## v0.4.4 - 2026-07-06

### 句子翻译：浮动窗口交互

- 顶部句子区不再平铺译文卡，而是渲染**原文分句**：每句 hover 高亮（动态过渡），
  点击某句在其位置弹出**浮动窗口**显示该句译文，并可在窗口内收藏（kind = sentence）。
- 单词卡、native 用法卡保持不变，仍在句子区下方；句子收藏功能保留。
- 浮层弹性宽度：默认较宽，随内容增长到 560px 上限后换行；内容超高时窗口内滚动，
  收藏按钮固定在底部始终可见。

### 通用 Popover 组件（可复用锚定浮层）

- 抽出 `ui/src/components/Popover.tsx`，把浮层定位/关闭逻辑组件化，按协议接收内容：
  调用方给出 `anchor`（触发点位置）+ `children`（可滚动主体）+ 可选 `footer`（固定底部）。
- **智能定位**：默认在锚点下方弹出，下方空间不足且上方更宽敞时**自动向上翻转**（箭头随之翻到底部）；
  水平方向夹住不出屏；箭头始终指向锚点；点击外部 / Esc / 滚动 / 缩放自动关闭。
  兼容 web 端 CSS zoom（坐标按 uiScale 归一化）。
- 句子翻译浮窗即基于它实现；见下方 Roadmap，未来阅读模式的点击划词翻译将复用同一套。

### 发布 / 安装

- **Linux 分发 native 二进制**：CI 额外打包裸二进制 tarball 上传 Release，install.sh 优先安装
  native 版（用系统 webkit2gtk），规避 AppImage 自带图形库在 Wayland/新 GPU 上的
  `EGL_BAD_PARAMETER` 崩溃；仅在缺失时 fallback AppImage（wrapper 设 WEBKIT 环境变量兜底），
  支持 `GLOSSA_FORCE_APPIMAGE=1` 强制。
- **Linux 桌面集成**：install.sh 安装后生成 `~/.local/share/applications/glossa.desktop`
  并从仓库下载 `icon.svg` 到图标目录 —— Glossa 进应用菜单并显示图标（此前只丢了个可执行文件，
  无 .desktop 故无图标）。
- **macOS 安装**改用 `hdiutil -mountpoint` 显式挂载点，根治 `/Volumes` 重复挂载导致的
  `cp: N/Glossa.app: No such file or directory`，并消除挂载 deprecation 提示。

### Roadmap / TODO

- **书籍翻译阅读学习**：计划接入 epub / 网页正文提取脚本，把整本书/文章导入为可阅读文本；
  阅读时点击句子或划词，用**同一套 `Popover` 组件**就地弹出翻译与学习卡片（词卡 / native 用法 /
  收藏），把"翻译器"扩展成"沉浸式阅读 + 生词沉淀"。Popover 的协议化设计（anchor + children + footer）
  正是为此预留的复用点。

## v0.4.2 - 2026-07-04

### 移动端修复

- 用 `100dvh` 替代 `100vh`：Safari 等移动浏览器的动态工具栏不再遮挡底部输入栏。
- 弹窗 `z-index` 提到 95（高于抽屉侧边栏 90）；窄屏下打开生词本/设置时自动收起抽屉侧边栏，
  不再被遮挡。

### 发布 / 安装

- **一行安装脚本** `install.sh`（Linux AppImage / macOS dmg，`curl … | sh`）：
  拉取最新 Release、原子替换，**重复运行即更新**。Windows 仍走安装包。
- **应用内更新提示**：桌面端启动时静默比对 GitHub 最新 Release，有新版显示可关闭的横幅
  （离线/限流静默跳过；不自动下载，尊重用户选择）。
- 修复 Windows CI 构建：`npm install` 改用 `working-directory: ui`
  （`npm --prefix ui` 在 Windows 上会误在仓库根找 package.json）。

### 品牌 / 发布

- 软件内 logo 改为**词标**：G 字形（Logo 组件）直接替换首字母，与 "lossa" 连成 Glossa，
  去掉原来的方块 badge 背景。
- README 重写为介绍版：顶部图标 hero、平台徽章、mermaid 架构图、加粗全平台优势。
- 修复 Release 无安装包/无说明：补 `LICENSE`（MIT）；CI 用实际 push 的 tag 名建 Release，
  附带安装说明 `releaseBody`；push `v*` tag 即产出三平台安装包。

## v0.4.0 - 2026-07-04

### 新增：Web 支持（docs/v0.4-web-architecture 落地）

- **kernel::AppCore**：把原先 src-tauri 里的业务胶水（config/store/memory/agent 组装）下沉为
  kernel 的统一门面，Tauri commands 与 Web handlers 都是薄适配层（src-tauri/state.rs 删除）。
- **crates/server（glossa-server）**：axum HTTP 适配层。
  - REST：/api/config、/api/sessions（CRUD + rename）、/api/memory（mark/unmark）。
  - 流式消息：POST /api/sessions/{id}/messages 返回 **NDJSON 流**（每行一个 SendEvent）。
    选 NDJSON 而非 SSE：EventSource 只支持 GET，fetch+ReadableStream 逐行解析更简单且移动端可用。
  - 前端静态资源经 rust-embed **内嵌进二进制**（同源服务，无 CORS）；assets 带 immutable 缓存，
    未知路径回退 index.html。
  - **安全模型**：默认绑 127.0.0.1 免鉴权；绑非回环地址（LAN/手机）必须设 GLOSSA_TOKEN
    （config 接口含 API key，不能裸奔进局域网），前端用 `?token=` 一次性传入后存 sessionStorage
    并自动携带 Bearer 头。
- **前端双后端抽象**：api.ts 拆为 Tauri（invoke/Channel）与 HTTP（fetch/NDJSON）两个实现，
  运行时按 `__TAURI_INTERNALS__` 自动选择，同一个 bundle 同时服务桌面与浏览器。

### Web 端 UI 差异化

- 不渲染自绘标题栏；不应用窗口圆角，自然铺满网页（`body.platform-web` 覆盖）。
- 不启用应用内 zoom（交给浏览器缩放），设置面板隐藏缩放项。
- 桌面基准尺寸在网页里偏大：web 端整体按 0.85 缩放（CSS zoom + 100vh 补偿），
  fixed 弹层（下拉/右键菜单）坐标除以 uiScale 修正。
- web 端保留浏览器原生右键菜单（桌面端仍屏蔽）。

### 侧边栏折叠（桌面 + Web）

- 侧边栏右上角「«」收起；收起后「☰」浮动在聊天区左上角展开；状态存 localStorage。
- **移动端适配**：≤768px 侧边栏变抽屉式覆盖层（带遮罩，选中会话自动收起，默认收起），
  气泡/卡片放宽到全宽、输入栏紧凑化、窄屏 placeholder 缩短。

### 统一 CLI 与内嵌 Web 服务

- `glossa` / `glossa app` → 桌面端；`glossa web` → Web 服务（glossa-server 改为 lib + bin，
  桌面二进制直接复用 router/serve，`web` 子命令不拉起 GUI）。
- Web 默认 **0.0.0.0:8040**：局域网无需 `--host` 即可访问，`--port` 指定端口；
  未设 GLOSSA_TOKEN 时不再拒绝启动，改为打印告警（token 鉴权仍可用）。
- 配置新增 `[web] enabled / port`：桌面端可随启内嵌 Web 服务；
  设置面板新增「Web 服务」段（默认关闭/默认开启切换 + 端口），保存立即生效
  （启停/换端口热切换，bind 失败直接报错到设置界面）。
- 侧边栏启动始终默认展开（不再记忆折叠状态；窄屏抽屉仍默认收起）。
- 修复：折叠侧边栏后主区被 Grid 自动放置挤进 0 宽列——sidebar/main 改为显式指定列。
- 终端输出：开启/关闭事件均有打印；不再显示 0.0.0.0，改为「本机 http://127.0.0.1:端口/」
  与「局域网 http://真实IP:端口/」可点击链接，标签按 CJK 显示宽度对齐。
- 局域网 IP 改为枚举网卡（跳过 tun/docker 等虚拟设备、优先私网 IPv4）——
  UDP 路由探测在 Clash TUN 下会误报 198.18.x fake-ip。
- 设置项文案：「开启 Web 服务：开启/关闭」。

### 验证

- `cargo test -p kernel` 29 passed；`cargo check` 全 workspace（含 glossa-server，axum 0.8）。
- `glossa --help` / `glossa web --port 8043` 冒烟：告警输出、静态页 200、config 含 web 段默认值。
- glossa-server 冒烟：静态页 200、config/sessions/memory REST 全通、rename/delete、SPA 回退、
  NDJSON 流式收到真实模型 delta（端到端含 LLM）。
- headless Chromium 截图验证 web 桌面（1500×900，无圆角/无标题栏/折叠按钮）与
  移动视口（414×896，抽屉收起 + ☰ 浮动按钮）。


## v0.3.0 - 2026-07-04

### 重构：翻译结果 schema（逐句对照 + 独立 native 表达卡）

- schema 新增 `sentences: [{src, dst}]` 逐句对照；多句时逐句渲染（左侧竖线原句 + 译文），
  单句仍显示整段译文。`translation` 与 `sentences` 均为空视为解析失败，触发修复重试。
- **native 用法语义修正**：不再是每个单词的附属字段，而是独立的 `usages` 卡片——
  仅当原句中确实出现值得学习的 native 表达（习语/固定搭配/句式）时输出（0~3 个，宁缺毋滥），
  每个 usage 带中文讲解 + 例句对照，卡片带 Native 徽标与「◇ 用法」标记按钮。
- word 卡只保留「☆ 生词」按钮（词与用法彻底分离）；每个词要求 1~2 条例句对照。
- `WordEntry.native_usage` 保留为 legacy 字段，仅用于渲染旧会话；新 prompt schema 不再包含。
- 验证：`cargo test -p kernel`（29 passed，含 sentences/usages 解析与空结果拒绝测试）、
  `npm --prefix ui run build`、`cargo check -p glossa`；另注入新 schema 预览会话启动截图确认渲染。

### 修复（v0.2.0 审查项）

- 最小窗口尺寸 1100×800 → 960×640：1366×768 等小屏放不下 800 高，且无边框窗口没有系统兜底。
- macOS 补 `app.macOSPrivateApi: true`：否则 `transparent: true` 在 macOS 不生效，圆角四角为实色。
- legacy 配置迁移：v0.1 的 `profile.effort`（现为 `#[serde(skip)]` 运行时字段，TOML 中会被静默忽略）
  自动迁移为 `chat_effort` 并回写配置文件。
- prompt 残留语义修正：删除"反复标记的词说明其真实水平"（`marked_count` 已移除，记录里没有次数）。

### 新增

- `Profile.provider`（`deepseek` | `openai`）显式指定 thinking 兼容层，缺省仍按 base_url 嗅探；
  设置面板提供下拉，聚合商 URL 含 "deepseek" 误判时可显式覆盖。
- 生词本条目支持删除（调用已有 unmark 命令）。
- **句子收藏**：新增 `MarkKind::Sentence`，句对卡带「☆ 句子」按钮，`word` 存原句、`meaning` 存译文；
  生词本以青色「句子」chip 展示，可搜索、可删除，并进入 memory 上下文供模型判断用户句式水平。

### UI 打磨

- 卡片体系：词卡「词」徽标 + 青色左线，Native 卡绿色，句卡主题色；词卡排在 Native 卡之前；
  句卡原句 20px 主色、译文 19px 次色。
- 输入栏三件套初始等高对齐，圆角统一 10px。
- 输入栏按钮不再随多行 textarea 拉伸；模式切换和发送按钮初始高度与一行输入框对齐，多行输入时只保持底部对齐。
- 侧边栏 18px 字号；生词本图标改为与 ⚙ 同风格的 Nerd Font 书本字形并等宽对齐。
- 主题下拉顺序调整为 Gruvbox Light / Gruvbox Dark / Catppuccin Light / Catppuccin Dark；默认主题改为 Gruvbox Light，裸 `:root` 也映射到 Gruvbox Light 以避免启动前闪回旧暗色默认。
- **自绘 Dropdown 组件**替换全部原生 `<select>`（WebKitGTK 弹出层无法用 CSS 主题化）：
  fixed 定位逃出弹窗滚动裁剪、最大高度 60vh、下方空间不足自动向上展开、选中项主题色高亮。
- **主题化右键菜单**：全局屏蔽 webview 浏览器菜单（输入框保留粘贴菜单）；
  会话 tab 右键出「重命名/删除」自绘菜单，✎ 与 × 按钮移除（双击改名保留）。
- 重命名不再更新 `updated` 时间戳，改名后 tab 不会跳到列表顶部（kernel 层修复 + 测试）。
- **删除确认**：删除会话、删除生词本条目均弹出主题化确认对话框（wry 无原生 confirm），
  Esc 取消 / Enter 确认。
- memory 上下文修复：usage 条目讲解存于 `native_usage`，序列化时回退填入 `meaning`，
  不再给模型发 null；system prompt 说明三类 kind 的含义。
- 生词本展开内容从“原句 context”改为“词卡/用法卡例句”：
  - 原设计会把用户标记单词或 native 用法时所在的整句 `context` 存进 memory，但对单词和固定表达来说，这个上下文常常只是本轮翻译原句，长期放在生词本里学习价值不稳定。
  - `MarkInput` / `VocabEntry` 新增 `examples: Example[]`，标记 word 时保存 `WordEntry.examples`，标记 native usage 时保存 `UsageEntry.examples`。
  - word / usage 标记不再传 `context`；新写入的 `VocabEntry.contexts` 固定为空，仅保留字段用于兼容 v0.2 旧数据反序列化。
  - 生词本搜索范围移除 `contexts`，改为搜索 `examples.en` + `examples.zh`；搜索提示改为“模糊搜索单词、释义、native 用法或例句…”。
  - 生词本列表不再渲染 `.vocab-context` / `.vocab-contexts-open`；展开入口改为 `展开例句（N）`，展开后显示例句标题、英文例句与中文翻译。
  - 这样 word / usage 的长期记忆内容从“出现过的原句”变为“可复习的教学例句”，与 v0.3 的词卡和独立 Native 用法卡语义一致。

## v0.2.0 - 2026-07-03

### 修复：安装版启动白屏

- 修复通过 `cargo install --path src-tauri` 安装后的 Glossa 启动白屏问题。
- 根因：安装版没有启用 Tauri `custom-protocol` 时，会按 dev-url 模式尝试加载 `http://localhost:1420`，没有 Vite dev server 时窗口为空白。
- 处理：
  - `src-tauri/Cargo.toml` 新增默认 feature：
    - `default = ["custom-protocol"]`
    - `custom-protocol = ["tauri/custom-protocol"]`
  - `ui/vite.config.ts` 新增 `base: "./"`，让打包后的前端资源使用相对路径，适配 Tauri custom protocol。
  - `src-tauri/build.rs` 监听 `../ui/dist/index.html` 与 `../ui/dist/assets`，避免 Cargo 复用旧的嵌入式前端资源。
  - `README.md` 更新系统命令安装方式，明确使用 `src-tauri` 作为安装路径，并说明前端需要先构建。
- 验证：
  - `npm --prefix ui run build`
  - `cargo install --path src-tauri --locked --features custom-protocol --force --root /tmp/glossa-install-test`
  - 启动 `/tmp/glossa-install-test/bin/glossa`，确认安装版 UI 正常显示，不再白屏。
  - `cargo check -p glossa`
  - `cargo check -p glossa --no-default-features`

### 优化：主题、窗口与安装文档

- 完善 Gruvbox Dark / Gruvbox Light 主题映射，使其更贴近官方 Gruvbox 暖暗背景、纸感前景与 yellow/orange accent。
- 完善 Catppuccin Mocha / Latte 的 base、mantle、surface、border、accent 层级。
- 调整默认窗口尺寸：`1360 × 900`，最小尺寸 `900 × 620`。
- 扩大侧边栏、翻译块、聊天块与设置弹窗的默认展示空间。
- 配置模板和设置界面明确展示 `[ui] zoom`。
- `README.md` 新增 Linux 下通过 Cargo 安装为 `glossa` 系统命令的步骤。
- 验证：
  - `npm --prefix ui run build`
  - `cargo test -p kernel`
  - `cargo check -p glossa`

### 新增：生词本面板

- 左侧侧边栏新增 `📚 生词本` 按钮。
- 新增 `ui/src/components/VocabBook.tsx`，直接复用前端已有的 `memory.words` 数据。
- 生词本默认按后端 `Vec<VocabEntry>` 的原始顺序展示，即添加顺序。
- 支持切换为字母顺序排序。
- 支持模糊搜索，搜索范围包括：
  - 单词 `word`
  - 释义 `meaning`
  - native 用法 `native_usage`
  - 上下文 `contexts`
  - 音标 `ipa`
  - 词性 `pos`
- 搜索结果按匹配度排序，并使用原始添加顺序作为稳定 tie-break。
- 生词本列表支持弹窗内滚动，避免条目过多时撑出窗口。
- 验证：
  - `npm --prefix ui run build`

### 修正：生词标记语义统一为状态

- 明确当前产品语义：生词标记是布尔状态，而不是次数累计。
- 删除后端 `VocabEntry.marked_count` 字段。
- 修改 `MemoryStore::mark`：
  - 未存在时插入记录。
  - 已存在时按 `kind + word` 大小写不敏感合并，并刷新 metadata/context，不再累计次数。
- 修改 prompt context：
  - 删除 `count` 字段。
  - 文案从 `标记记录（count 越大越说明掌握薄弱）` 改为 `最近标记记录`。
- 更新前端 `VocabEntry` 类型，移除 `marked_count`。
- 生词本 UI 不再显示 `×1`、`1 次` 或 `添加` 字样。
- 生词本日期显示改为简短格式，例如 `7/3`。
- 验证：
  - `cargo test -p kernel`
  - `npm --prefix ui run build`
  - `cargo check -p glossa`


### 新增：翻译/聊天分别配置思考模式

- 将思考模式拆为翻译和聊天两个独立设置，不再共用。
- 每个模式一个下拉，同时包含 `no thinking` 与 effort 等级：
  - `no thinking`（不启动思考链）
  - `low` / `medium` / `high` / `xhigh`
- 数据结构：
  - Profile 新增 `translate_effort` / `chat_effort`（持久化字段），删除原有的 `effort` / `thinking` 字段。
  - 新增瞬态字段 `effort`（`#[serde(skip)]`），由 `agent.rs` 根据当前 mode 从对应字段取值后传入 `client.rs`。
- `client.rs` 映射（含 provider 兼容层）：
  - `base_url` 含 `deepseek` → 发送 `thinking` 字段，`effort=None` 时 `disabled`，`Some` 时 `enabled`
  - `base_url` 不含 `deepseek` → 不发送 `thinking` 字段，仅标准 `reasoning_effort`；`xhigh` 自动 normalise 为 `high`
- 新增 5 个 kernel 测试：
  - `no_effort_sends_thinking_disabled_without_reasoning`
  - `effort_low_sends_thinking_enabled_and_reasoning`
  - `effort_xhigh_sends_thinking_enabled_and_reasoning`
  - `openai_no_thinking_field_and_xhigh_normalised`
  - `openai_no_effort_sends_nothing`
- 验证：
  - `cargo test -p kernel` — 22 passed（含 5 个新增）
  - `npm --prefix ui run build`
  - `cargo check -p glossa`


### 调整：provider 兼容层

- `client.rs` 新增 `base_url` 自动检测：含 `deepseek` 才发送 `thinking` 字段，非 DeepSeek provider 只发标准 `reasoning_effort`。
- 非 DeepSeek 时 `xhigh` 自动 normalise 为 `high`（OpenAI 不接受 xhigh）。
- 新增 2 个 OpenAI 路径测试。

### 调整：UI 标签与默认思考设置

- UI 中"严格翻译"统一改为"翻译"（InputBar / Conversation / Settings）。
- 系统 prompt 中"严格翻译模式"保持不变（模型输出约束）。
- 默认思考设置：翻译模式 no thinking，聊天模式 xhigh。
- 配置文件模板中 `translate_effort` 默认注释掉，`chat_effort` 默认 `"xhigh"`。
- 新增断言 `translate_effort.is_none()` 防止回归。



### 新增：硬编码 CodeNewRoman Nerd Font

- 从 Nerd Fonts v3.4.0 引入 CodeNewRoman Nerd Font，SIL OFL 1.1 许可证。
- 转换为 WOFF2，bundled 进 ui/src/assets/（Regular / Bold / Italic，共约 6MB）。
- body 字体栈第一优先级设为 `"CodeNewRoman"`，回退系统字体。

### 调整：基准字号与默认窗口尺寸

- body 基础字号 14px → 20px，`.word` 20px，其余元素等比放大（translation 21px，hint-title 24px 等）。
- zoom 默认值重置为 1.0（基准字号已足够大，不再依赖默认缩放）。
- 默认窗口 1360×900 → 1640×1050，最小 900×620 → 1100×800。
- 侧边栏 232px → 272px → 320px → 420px → 400px。
- 窗口圆角从“内容区圆角”改为“窗口表面圆角”：
  - Tauri 窗口配置新增 `"transparent": true`、`"decorations": false`、`"shadow": true`。
  - `transparent` 让 WebView 背后的原生窗口背景可透出，不再由系统默认方形背景填满四角。
  - `"decorations": false` 去掉系统标题栏/边框，避免系统装饰层仍然保持方形外框。
  - 前端将 `html` / `body` / `#root` 背景设为 `transparent`，防止 WebView 根背景重新把透明窗口四角涂成方形。
  - `.app` 成为真正的窗口视觉表面：`background: var(--bg)` + `border-radius: 14px` + `overflow: hidden`。
  - 自定义标题栏接管原生标题栏职责，包含拖拽区域和最小化/最大化/关闭按钮。
  - Tauri v2 capability 明确开放 `core:window:allow-minimize`、`core:window:allow-toggle-maximize`、`core:window:allow-close`、`core:window:allow-start-dragging`，修复 Linux 下自定义窗口按钮/拖拽被权限系统拦截的问题。
  - 权限配置在 Tauri 启动时加载，测试右上角按钮时需要完全退出并重启应用；仅靠前端热更新可能仍然使用旧 capability。
  - `.modal-backdrop` 从 `position: fixed` 改为 `position: absolute`，并依赖 `.app { position: relative; overflow: hidden; }`，确保设置弹窗/生词本遮罩也被外层 14px 圆角裁切。
  - 注意：Linux/Windows/macOS 的最终圆角显示仍取决于窗口管理器/合成器对透明无边框窗口的支持；构建验证只能证明配置和前端代码有效，视觉效果需要实际启动确认。

## TODO


- 严格翻译是否不应该携带聊天上下文？不要让上下文干扰需要翻译的内容？还是提示词注明不要翻译上下文？因为上下文有助于下一轮翻译？但上下文太多影响翻译速度？




