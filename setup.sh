#!/bin/bash
# Claude Code Statusline 一键安装脚本
# 从 GitHub Release 下载并配置

set -e

REPO="zzfn/cc-statusline"
BINARY_NAME="cc-statusline"
INSTALL_DIR="${1:-$HOME/.claude}"

# 检测系统架构
detect_platform() {
    local os arch
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) echo "不支持的架构: $arch"; exit 1 ;;
    esac

    case "$os" in
        darwin) os="apple-darwin" ;;
        linux) os="unknown-linux-gnu" ;;
        *) echo "不支持的系统: $os"; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

# 获取最新 release 版本
get_latest_version() {
    curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
}

echo "=== Claude Code Statusline 安装脚本 ==="
echo ""

# 检测平台
PLATFORM=$(detect_platform)
echo "检测到平台: $PLATFORM"

# 获取最新版本
echo "获取最新版本..."
VERSION=$(get_latest_version)

if [ -z "$VERSION" ]; then
    echo "警告: 无法获取最新版本，使用 latest"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}-${PLATFORM}.tar.gz"
else
    echo "最新版本: $VERSION"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY_NAME}-${PLATFORM}.tar.gz"
fi

# 创建安装目录
echo "创建安装目录: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"

# 下载并解压
echo "下载中..."
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

if curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/release.tar.gz"; then
    tar -xzf "$TMP_DIR/release.tar.gz" -C "$TMP_DIR"
    mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
else
    # 如果 tar.gz 不存在，尝试直接下载二进制
    DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}-${PLATFORM}"
    echo "尝试直接下载二进制..."
    curl -fsSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
fi

echo "已安装到: $INSTALL_DIR/$BINARY_NAME"

# 配置 settings.json
SETTINGS_FILE="$INSTALL_DIR/settings.json"
STATUSLINE_CONFIG='"statusLine": {"type": "command", "command": "~/.claude/cc-statusline", "padding": 0}'

if [ -f "$SETTINGS_FILE" ]; then
    if command -v jq &> /dev/null; then
        TMP_FILE=$(mktemp)
        jq '.statusLine = {"type": "command", "command": "~/.claude/cc-statusline", "padding": 0}' "$SETTINGS_FILE" > "$TMP_FILE"
        mv "$TMP_FILE" "$SETTINGS_FILE"
        echo "已更新配置: $SETTINGS_FILE"
    else
        if grep -q '"statusLine"' "$SETTINGS_FILE"; then
            echo "配置已存在，请手动检查 $SETTINGS_FILE"
        else
            echo "请手动添加以下配置到 $SETTINGS_FILE:"
            echo "  $STATUSLINE_CONFIG"
        fi
    fi
else
    cat > "$SETTINGS_FILE" << 'EOF'
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/cc-statusline",
    "padding": 0
  }
}
EOF
    echo "已创建配置: $SETTINGS_FILE"
fi

echo ""
echo "✓ 安装完成！"
echo ""
echo "重启 Claude Code 或配置会自动生效。"
