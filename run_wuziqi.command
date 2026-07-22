#!/bin/bash
# 双击运行:用 Rust 启动五子棋 (wuziqi)
# 若未安装 Rust 会先通过官方 rustup 自动安装
set -uo pipefail

cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

if ! command -v cargo >/dev/null 2>&1; then
  echo "未检测到 Rust 工具链,即将从官方源 (rustup.rs) 自动安装。"
  echo "安装只影响当前用户目录 (~/.cargo 和 ~/.rustup),不需要管理员密码。"
  read -p "按回车开始安装 (Ctrl+C 取消)..."
  installer="$(mktemp -t wuziqi-rustup.XXXXXX)" || exit 1
  trap 'rm -f "$installer"' EXIT
  if ! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$installer"; then
    echo "❌ 无法下载 Rust 官方安装程序。"
    read -p "按回车键关闭..."
    exit 1
  fi
  if ! sh "$installer" -y --default-toolchain stable; then
    echo "❌ Rust 安装失败。"
    read -p "按回车键关闭..."
    exit 1
  fi
  rm -f "$installer"
  trap - EXIT
  source "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ Rust 安装未成功，请保留上面的错误信息。"
  read -p "按回车键关闭..."
  exit 1
fi

echo ""
echo "🎮 正在编译并启动五子棋 (首次编译需下载依赖,可能要几分钟)..."
if ! cargo run --release; then
  echo "❌ 编译或运行失败，请保留上面的错误信息。"
  read -p "按回车键关闭..."
  exit 1
fi
