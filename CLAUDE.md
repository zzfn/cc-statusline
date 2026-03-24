# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目简介

这是一个用 **Rust** 实现的 Claude Code 状态栏（statusline）插件。它通过 stdin 接收 Claude Code 传入的 JSON 数据，格式化后输出到 stdout，在终端显示模型、上下文使用率、缓存命中率、Git 分支等信息。支持可选的第三方 API 配额展示（ZAI、云逸）。

## 常用命令

```bash
# 构建
cargo build --release

# 运行测试
cargo test

# 运行单个测试
cargo test test_get_dir_name

# 本地安装（构建并复制到 ~/.claude/cc-statusline）
./install.sh
```

## 架构

代码分为两个核心文件：

### `src/main.rs`

主程序，负责整个 statusline 的构建流程：

1. 从 **stdin** 读取 JSON（`StatusInput` 结构体，含 model、workspace、context_window、cost 等字段）
2. 读取 `~/.claude/settings.json` 获取 `apiKey` / `apiBaseUrl`（用于第三方 provider 鉴权）
3. 调用 `build_statusline()` 拼接各段输出
4. 输出到 **stdout**

关键函数：`build_statusline()`、`get_git_branch()`、`get_context_color()`、`calculate_cache_hit_rate()`

### `src/providers.rs`

第三方 API 配额集成，定义了 `Provider` trait：
```rust
pub trait Provider {
    fn name(&self) -> &'static str;
    fn matches(&self, base_url: &str) -> bool;
    fn get_parts(&self, base_url: &str, auth_token: &str) -> Vec<String>;
}
```

当前实现：
- **ZhipuProvider**（z.ai / bigmodel.cn）：展示 Token 和 MCP 调用使用率，3 分钟本地缓存
- **YunyiProvider**（云逸）：展示日剩余额度、过期时间、配额包信息，1 分钟本地缓存

### 添加新 Provider 的步骤
1. 在 `providers.rs` 中实现 `Provider` trait
2. 在 `get_provider()` 函数中注册新 provider

## 构建优化

`Cargo.toml` 的 release profile 配置了：`opt-level = "z"`（最小体积）、LTO、strip——目标是最小化二进制文件大小。

## 发布流程

推送 `v*` 标签后，GitHub Actions（`.github/workflows/release.yml`）自动跨平台编译并发布 Release，覆盖 Linux/macOS/Windows 的 x86_64 和 ARM64。
