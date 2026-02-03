# Claude Code Statusline (Rust)

一个用 Rust 实现的 Claude Code statusline 插件，显示模型、目录、上下文使用率、成本等信息。

## 效果预览

```
[Opus] │ my-project │ main │ 📝3 │ ctx:42% │ in:15.2k │ cache:85% │ $0.012 │ ⏱15m │ +156/-23
```

## 安装

### 方式一：一键安装（推荐）

```bash
curl -fsSL https://raw.githubusercontent.com/zzfn/cc-statusline/main/setup.sh | bash
```

### 方式二：从源码构建

```bash
git clone https://github.com/zzfn/cc-statusline.git
cd cc-statusline
./install.sh
```

### 方式三：手动安装

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

## 显示内容

| 项目 | 说明 | 颜色 |
|------|------|------|
| `[Model]` | 当前模型名称 | 紫色 |
| 目录名 | 当前工作目录 | 青色 |
| Git 分支 | 当前 git 分支 | 蓝色 |
| `📝N` | 未提交的文件数 | 黄色 |
| `ctx:N%` | 上下文窗口使用率 | 绿/黄/红 |
| `in:Nk` | 输入 token 数 | 灰色 |
| `cache:N%` | 缓存命中率 | 绿/黄/红 |
| `$N.NN` | 会话成本 (USD) | 黄色 |
| `⏱Nm` | 会话时长 | 青色 |
| `+N/-N` | 代码行变更 | 绿/红 |

上下文使用率颜色：
- 绿色: < 60%
- 黄色: 60-80%
- 红色: > 80%

缓存命中率颜色：
- 绿色: ≥ 80%
- 黄色: 50-80%
- 红色: < 50%

## 自定义

修改 `src/main.rs` 中的 `build_statusline` 函数来自定义显示内容。

## License

MIT
