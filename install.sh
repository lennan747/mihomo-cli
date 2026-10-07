#!/usr/bin/env sh
#
# mihomo-cli 一键安装脚本
#
# 用法：
#   curl -fsSL https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.sh | sh
#
# 环境变量：
#   MIHOMO_CLI_VERSION       指定版本（默认 latest，如 v0.3.0）
#   MIHOMO_CLI_INSTALL_DIR   安装目录（默认 $HOME/.local/bin）
#
# 安全：下载后强制校验发布资产对应的 SHA256SUMS。

set -eu

REPO="lennan747/mihomo-cli"
PROJECT="mihomo-cli"

log()  { printf '\033[1;32m[%s]\033[0m %s\n' "$PROJECT" "$1"; }
warn() { printf '\033[1;33m[%s]\033[0m %s\n' "$PROJECT" "$1"; }
die()  { printf '\033[1;31m[%s]\033[0m %s\n' "$PROJECT" "$1" >&2; exit 1; }

# --- 1. 识别系统与架构，映射到发布资产 target -------------------------------
detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Linux)
            case "$arch" in
                x86_64|amd64) echo "x86_64-unknown-linux-gnu" ;;
                *) die "不支持的 Linux 架构: $arch（当前仅提供 x86_64）" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                arm64) echo "aarch64-apple-darwin" ;;
                *) die "暂不支持的 macOS 架构: $arch（当前仅提供 arm64；macOS Intel 请用 cargo 安装）" ;;
            esac
            ;;
        *)
            die "不支持的操作系统: $os（Linux/macOS 可用；Windows 请用 PowerShell: irm https://raw.githubusercontent.com/lennan747/mihomo-cli/master/install.ps1 | iex）"
            ;;
    esac
}

TARGET="$(detect_target)"
log "目标平台: $TARGET"

# --- 2. 解析版本 -------------------------------------------------------------
VERSION="${MIHOMO_CLI_VERSION:-latest}"
if [ "$VERSION" = "latest" ]; then
    log "查询最新版本 ..."
    VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
    [ -n "$VERSION" ] || die "无法获取最新版本号"
fi
log "版本: $VERSION"

# --- 3. 下载并校验 -----------------------------------------------------------
ASSET="$PROJECT-$TARGET.tar.gz"
BASE="https://github.com/$REPO/releases/download/$VERSION"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

log "下载 $ASSET ..."
curl -fsSL -o "$TMP/$ASSET" "$BASE/$ASSET" || die "下载失败: $BASE/$ASSET"

log "下载并校验 SHA256SUMS ..."
curl -fsSL -o "$TMP/SHA256SUMS" "$BASE/SHA256SUMS" || die "下载 SHA256SUMS 失败"
# SHA256SUMS 包含全部平台资产，只提取本资产所在行进行校验
grep " $ASSET$" "$TMP/SHA256SUMS" > "$TMP/checksums.txt" \
    || die "SHA256SUMS 中未找到 $ASSET 的校验值"
(cd "$TMP" && sha256sum -c checksums.txt) || die "SHA256 校验失败，已中止安装（请勿绕过校验）"
log "校验通过"

# --- 4. 解压并安装 -----------------------------------------------------------
INSTALL_DIR="${MIHOMO_CLI_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP/$ASSET" -C "$TMP"
install -m 0755 "$TMP/$PROJECT" "$INSTALL_DIR/$PROJECT"

log "已安装: $INSTALL_DIR/$PROJECT ($VERSION)"
if ! command -v "$PROJECT" >/dev/null 2>&1; then
    warn "$INSTALL_DIR 不在 PATH 中，请将其加入 PATH：export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
"$INSTALL_DIR/$PROJECT" --version
