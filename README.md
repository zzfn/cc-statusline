# Claude Code Statusline (Rust)

一个用 Rust 实现的 Claude Code statusline 插件，显示模型、目录、上下文使用率、缓存命中率、Git 分支等信息。支持 Anthropic 官方 API 用量展示和第三方 API 配额展示（ZAI）。

## 效果预览

基础显示：
```
[Claude Sonnet 4.5] │ my-project │ main │ ctx:42% │ cache:85%
```

使用 Anthropic 官方 API 时额外显示（重置时间当天只显示时刻，否则显示日期+时刻）：
```
[Claude Sonnet 4.5] │ my-project │ main │ ctx:42% │ cache:85% │ current  42% 10:30pm │ weekly   18% Mar 30, 10:30pm
```

Claude 2x 双倍用量激活时末尾追加：
```
... │ 2x⚡(2h 25m)
```

## 安装

### 方式一：一键安装（推荐）

#### Linux/macOS

```bash
curl -fsSL https://raw.githubusercontent.com/zzfn/cc-statusline/main/setup.sh | bash
```

#### Windows

在 PowerShell 中运行（以管理员身份）：

```powershell
irm https://raw.githubusercontent.com/zzfn/cc-statusline/main/setup.ps1 | iex
```

或下载后运行：

```powershell
Invoke-WebRequest -Uri https://raw.githubusercontent.com/zzfn/cc-statusline/main/setup.ps1 -OutFile setup.ps1
.\setup.ps1
```

### 方式二：从源码构建

```bash
git clone https://github.com/zzfn/cc-statusline.git
cd cc-statusline
./install.sh  # Linux/macOS
# 或在 Windows 上使用: cargo build --release
```

### 方式三：手动安装

#### Linux/macOS

1. 从 [Releases](https://github.com/zzfn/cc-statusline/releases) 下载对应平台的二进制文件
2. 解压并复制到 `~/.claude/`
3. 在 `~/.claude/settings.json` 中添加：

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/cc-statusline",
    "padding": 0
  }
}
```

#### Windows

1. 从 [Releases](https://github.com/zzfn/cc-statusline/releases) 下载 `cc-statusline-x86_64-pc-windows-msvc.zip`
2. 解压到 `%USERPROFILE%\.claude\` 目录
3. 在 `%USERPROFILE%\.claude\settings.json` 中添加：

```json
{
  "statusLine": {
    "type": "command",
    "command": "%USERPROFILE%\\.claude\\cc-statusline.exe",
    "padding": 0
  }
}
```

## 显示内容

| 项目 | 说明 | 颜色 |
|------|------|------|
| `[Model]` | 当前模型名称 | 紫色加粗 |
| 目录名 | 当前工作目录（取最后一段） | 青色 |
| Git 分支 | 当前 git 分支 | 蓝色 |
| `ctx:N%` | 上下文窗口使用率 | 绿/黄/红 |
| `cache:N%` | 缓存命中率 | 绿/黄/红 |
| `current N% HH:MM` | 官方 API 5小时用量及重置时间 | 白色 |
| `weekly N% date` | 官方 API 7天用量及重置时间 | 白色 |
| `extra $X.XX/$X.XX date` | 官方 API 额外月度用量（启用时显示） | 白色 |
| `[ZAI] Token(5h):N%` | ZAI Token 使用率（5小时窗口） | 绿/黄/红 |
| `[ZAI] MCP(1月):N%` | ZAI MCP 调用使用率（1个月窗口） | 绿/黄/红 |
| `2x⚡(Xh Xm)` | Claude 双倍用量激活状态及剩余时间 | 绿色加粗 |

### 颜色规则

**上下文使用率 / ZAI 使用率：**
- 绿色: < 60%
- 黄色: 60–80%
- 红色: ≥ 80%

**缓存命中率：**
- 绿色: ≥ 80%
- 黄色: 50–80%
- 红色: < 50%

## Anthropic 官方 API 用量

使用 Anthropic 官方 API（OAuth 登录）时，程序会自动获取 5小时和7天用量，并可选显示额外月度用量。

OAuth token 读取顺序：
1. 环境变量 `CLAUDE_CODE_OAUTH_TOKEN`
2. macOS Keychain（`Claude Code-credentials`）
3. `~/.claude/.credentials.json`
4. Linux `secret-tool`

用量数据每 **60 秒**本地缓存一次（`~/.claude/.anthropic_usage_cache.json`）。

## ZAI 配额展示

说明：ZAI 为第三方服务，与 Claude/Anthropic 无官方关系。

如果你使用 ZAI API，只需在 `~/.claude/settings.json` 中配置 `baseURL` 和 `authToken`，或设置环境变量：

```bash
export ANTHROPIC_BASE_URL="https://api.z.ai/api/anthropic"
# 或
export ANTHROPIC_BASE_URL="https://open.bigmodel.cn/api/anthropic"

export ANTHROPIC_AUTH_TOKEN="your-token-here"
```

程序会自动检测并显示 ZAI 的 Token 使用率和 MCP 使用率。配额数据每 **3 分钟**本地缓存一次（`~/.claude/.zhipu_cache.json`）。

## Claude 2x 双倍用量

程序会自动从 [isclaude2x.com](https://isclaude2x.com) 检测当前是否处于 Claude 双倍用量窗口，并在激活时显示剩余时间。状态每 **5 分钟**本地缓存一次（`~/.claude/.claude2x_cache.json`）。

## 自定义

修改 `src/main.rs` 中的 `build_statusline` 函数来自定义显示内容。

添加新的第三方 API 配额展示：在 `src/providers.rs` 中实现 `Provider` trait，然后在 `providers()` 函数中注册。

## License

MIT
