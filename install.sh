#!/bin/sh
# Glossa 安装/更新脚本（Linux & macOS）
#   curl -fsSL https://raw.githubusercontent.com/xyt-dev/Glossa/main/install.sh | sh
# 幂等：重复运行即更新到最新 Release。
# Windows 请从 Releases 页面下载 .msi / -setup.exe。

set -eu

REPO="xyt-dev/Glossa"
API="https://api.github.com/repos/$REPO/releases/latest"

say() { printf '%s\n' "$*"; }
die() { printf '错误: %s\n' "$*" >&2; exit 1; }

command -v curl >/dev/null 2>&1 || die "需要 curl"

OS=$(uname -s)
ARCH=$(uname -m)

json=$(curl -fsSL "$API") || die "获取最新版本失败（GitHub API 不可达？）"
tag=$(printf '%s' "$json" | grep -m1 '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
[ -n "$tag" ] || die "未找到任何 Release"
say "最新版本: $tag"

urls=$(printf '%s' "$json" | grep '"browser_download_url"' | sed 's/.*"\(https[^"]*\)".*/\1/')

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64 | amd64) pat='(amd64|x86_64).*\.AppImage$' ;;
      aarch64 | arm64) pat='(aarch64|arm64).*\.AppImage$' ;;
      *) die "未支持的架构: $ARCH" ;;
    esac
    url=$(printf '%s\n' "$urls" | grep -E "$pat" | head -1 || true)
    # 未标注架构的 AppImage 视为 x86_64
    [ -n "$url" ] || url=$(printf '%s\n' "$urls" | grep -E '\.AppImage$' | head -1 || true)
    [ -n "$url" ] || die "Release 中没有 AppImage 资产"

    bin_dir="${GLOSSA_INSTALL_DIR:-$HOME/.local/bin}"
    app_dir="$HOME/.local/lib/glossa"
    app="$app_dir/glossa.AppImage"
    bin="$bin_dir/glossa"
    mkdir -p "$bin_dir" "$app_dir"
    say "下载 $(basename "$url") ..."
    curl -fL --progress-bar -o "$app.download" "$url"
    chmod +x "$app.download"
    # 原子替换：正在运行的旧版本不受影响，下次启动即新版。
    mv "$app.download" "$app"
    # 终端入口用 wrapper，而不是直接把 AppImage 命名为 glossa：
    # 1) ~/.local/bin 里只放命令入口，真实 AppImage 放到 ~/.local/lib/glossa；
    # 2) AppImage 内的 WebKitWebProcess 在部分 Linux 终端环境下会 EGL 崩溃，
    #    wrapper 只对 AppImage 启动补环境，不污染 cargo install / 开发运行。
    cat > "$bin.download" <<'EOF'
#!/bin/sh
set -eu
app="${GLOSSA_APPIMAGE:-$HOME/.local/lib/glossa/glossa.AppImage}"
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
exec "$app" "$@"
EOF
    chmod +x "$bin.download"
    mv "$bin.download" "$bin"
    say "已安装/更新: $app"
    say "命令入口: $bin（glossa = 桌面端，glossa web = Web 服务）"
    case ":$PATH:" in
      *":$bin_dir:"*) ;;
      *) say "提示: 请把 $bin_dir 加入 PATH" ;;
    esac
    ;;

  Darwin)
    url=$(printf '%s\n' "$urls" | grep -E '\.dmg$' | head -1 || true)
    [ -n "$url" ] || die "Release 中没有 dmg 资产"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    say "下载 $(basename "$url") ..."
    curl -fL --progress-bar -o "$tmp/glossa.dmg" "$url"
    mnt=$(hdiutil attach -nobrowse -readonly "$tmp/glossa.dmg" | awk '/\/Volumes\//{print $NF; exit}')
    [ -n "$mnt" ] || die "挂载 dmg 失败"
    rm -rf "/Applications/Glossa.app"
    cp -R "$mnt/Glossa.app" /Applications/
    hdiutil detach "$mnt" >/dev/null
    # 未签名应用去隔离标记，避免“已损坏”提示
    xattr -cr /Applications/Glossa.app 2>/dev/null || true
    say "已安装/更新: /Applications/Glossa.app"
    ;;

  *)
    die "此脚本支持 Linux / macOS；Windows 请从 https://github.com/$REPO/releases 下载安装包"
    ;;
esac
