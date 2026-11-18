# Architecture

```text
React / Tauri WebView
        |
        | typed commands + events
        v
Tauri 2 shell
        |
        v
yiyu-core (Rust)
```

## Layers

### `crates/yiyu-core`

不依赖 Tauri，负责：

- Provider 配置与 OpenAI-compatible 请求
- SSE 增量解析与非流式回退
- 语言识别与方向路由
- 内置与自定义翻译 Prompt
- 设置 schema、校验和原子写入
- SQLite 历史、收藏和保留上限
- HTTP 代理策略与 Release 资产选择

### `src-tauri`

负责所有平台能力：

- 主窗口、设置窗口、onboarding、快捷浮窗
- macOS 菜单栏、Windows 托盘
- 全局快捷键注册与冲突验证
- macOS Accessibility 和 Windows UI Automation 取词
- 恢复式剪贴板回退
- Keychain / Credential Manager
- macOS NSPanel 与 Windows non-activating window
- 更新下载、SHA256 校验和安装器启动

前端不能直接读取 Keychain、文件系统或执行任意命令。文件、凭据、网络和安装动作全部经 Rust command。

## Runtime flow

```text
hotkey
  -> permission check
  -> native selection
  -> clipboard fallback + restore
  -> language route
  -> OpenAI-compatible stream
  -> translation://event
  -> popup UI
  -> local history
```

## Storage

- `settings.json`: versioned, atomic JSON
- `history.sqlite`: WAL, `PRAGMA user_version` migrations
- `logs/`: tracing logs with redacted diagnostics
- `updates/`: resumable `.part` files and verified installers

## Contract

IPC types are mirrored in `web/src/lib/types.ts`, `contract.ts` and `src-tauri/src/dto.rs`. Command names are camelCase at the JavaScript boundary and snake_case Tauri command identifiers.
