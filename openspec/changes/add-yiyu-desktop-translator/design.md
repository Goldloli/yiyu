# Design

## Context

See `proposal.md` for motivation and the capability specs for behavior contracts. The repository is greenfield and already contains Git、OpenSpec、CodeGraph 与 Beads。可复用的工程基线参考 BiliDown 项目：Rust workspace core、Tauri 2 shell、React 前端、typed IPC、JSON 设置、SQLite 任务数据、严格 CI 与平台发布脚本。本设计沿用这些工程模式，但不复用下载器业务模型。

MoePeek 仅作为交互研究与行为参考，不作为源码依赖。其许可证为 AGPL-3.0；译羽采用 MIT，必须独立实现。

## Goals / Non-Goals

**Goals:**

- 用统一 Rust core 承载翻译、Provider、语言路由、设置与历史，使系统逻辑不依赖 Tauri。
- 在 macOS 与 Windows 上提供快速、可恢复的全局划词入口和非激活浮窗。
- 以 OpenAI-compatible 协议覆盖云端、自建和本地 LLM，保持 Provider 扩展空间。
- 前端保持轻量首屏、完整浅深色主题、停止式流式更新与可测试的 IPC 契约。
- 使全链路凭据、代理、日志、更新和剪贴板行为具备明确的安全边界。

**Non-Goals:**

- 不在本 change 中实现 OCR、自动划词浮标、TTS、多模型并行或额外 AI 任务。
- 不自行训练模型、不内置模型运行时、不提供项目共享密钥或代理。
- 不追求一套完全相同的跨平台原生外观；沿用各平台窗口与菜单惯例。
- 不在第一版解决正式签名、公证和商店分发。

## Decisions

### D1. Workspace 分层与依赖方向

```text
web (React UI)
      |
      | typed invoke/event contract
      v
src-tauri (Tauri shell)
      |
      v
crates/yiyu-core (domain + provider + storage)
```

新建 Rust workspace：

- `crates/yiyu-core`：不依赖 Tauri，包含翻译会话、OpenAI-compatible Provider、SSE 解析、Prompt、语言路由、设置模型、历史 DB 和错误映射。
- `src-tauri`：Tauri 2 外壳，包含窗口、托盘、全局快捷键、平台取词、凭据库、代理适配、更新器和 IPC。
- `web`：React 19 + TypeScript + Vite 8 + Tailwind CSS 4 + Zustand 5。

**Rationale**：核心可独立单元测试，平台壳可替换窗口和应用生命周期，不需要把业务逻辑塞进 Tauri command。相比单体 Rust crate，分层能让 Provider、历史和平台适配各自拥有清晰边界。

### D2. 多窗口与前端路由

前端保留单一构建产物，通过窗口 label 与 hash route 分离：

| Window label | Route | 职责 |
| --- | --- | --- |
| `main` | `#/main` | 历史侧栏 + 翻译工作台 |
| `popup` | `#/popup` | 快捷翻译浮窗 |
| `settings` | `#/settings` | Provider、网络、外观、更新、历史与权限设置 |
| `onboarding` | `#/onboarding` | 首次启动与权限/Provider 配置 |

每窗口共享 `contract.ts`、DTO adapter、tokens 和基础组件，但不共享不可序列化的 React state。Rust 通过 events 推动翻译流、设置 revision、更新状态和权限变化。

**Alternatives considered**：

- 全部功能放进一个主窗口：启动更重，不符合快捷浮窗和后台驻留定位。
- 每个窗口独立前端工程：构建重复、契约漂移风险高。

### D3. IPC 契约与状态边界

IPC 分为 commands 与 events：

- Commands：`get_bootstrap`、`translate_start`、`translate_cancel`、`capture_selection`、`translate_clipboard`、`patch_settings`、`save_provider_secret`、`test_provider`、`fetch_models`、history CRUD、update actions。
- Events：`translation://state`、`translation://delta`、`settings://changed`、`capture://permission`、`update://state`、`app://notice`。

所有字段 camelCase；Rust DTO 与 TypeScript contract 一一对应。Renderer 不能读取已保存的 API Key，只能提交新值或删除引用。设置使用 revision + patch，避免旧响应覆盖新值。

**Rationale**：沿用 bilidown 已验证的 mock/real backend 双实现模式，使浏览器截图、交互测试和生产 Tauri 适配共用 UI；事件与命令有稳定文本契约，便于 mock、真机和 CI 测试。

### D4. Tauri capabilities 最小授权

默认 capability 只向 `main`、`popup`、`settings`、`onboarding` 授权实际使用的窗口、事件、剪贴板、通知与 opener 权限。文件、数据库、Keychain、网络、下载和安装全部通过 Rust command 执行，不向前端开放通配文件系统或 shell。

**Rationale**：bilidown 已验证 capability 缺失会让窗口 API 静默失败；同时前端不应拥有任意命令执行或凭据读取能力。

### D5. 选区捕获适配层

选区捕获放在 `src-tauri` 的平台适配模块，不进入 React：

```text
hotkey
  -> permissions readiness
  -> native selection reader
       macOS: Accessibility / focused AX element
       Windows: UI Automation focused element + text pattern
  -> if unavailable and conservative=false:
       snapshot clipboard
       simulate Cmd+C / Ctrl+C
       wait for clipboard change with bounded timeout
       read selected text
       restore prior clipboard unless user changed it
  -> normalized text or classified failure
```

实现候选：

- macOS：`axuielement`（AXUIElement）与 `macos-accessibility-client` 权限检测。
- Windows：`uiautomation`（UI Automation）以及 `windows` crate 的原生扩展。
- 模拟复制：平台原生 CGEvent / SendInput，或经过 spike 验证的跨平台输入适配。

不直接依赖 MoePeek 的 Swift 实现。剪贴板回退的快照、变更检测、取消后等待和恢复必须由 Rust core or shell 中的独立接口实现并覆盖单元测试。

**Rationale**：只有“原生优先 + 恢复式回退”能兼顾隐私与覆盖率。相比仅模拟复制，它减少常规划词时对剪贴板的依赖；相比仅原生读取，它避免在浏览器、PDF 和 Electron 应用中大量失败。

### D6. 非激活浮窗

macOS 使用 `tauri-nspanel` 或受控的 `objc2` 桥接把 popup WebView 转为 `NSPanel`，使用 non-activating style、floating level、all-spaces/fullscreen auxiliary 和可点击后成为 key 的行为。Windows popup 使用无边框、透明、always-on-top、skip-taskbar 窗口，并通过 Win32 extended style 在不抢焦点与用户点击后可编辑之间切换。

每次显示前：

1. 读取触发点所在显示器与工作区。
2. 以光标附近为锚点定位，必要时翻转方向并 clamp 到安全区域。
3. 非激活显示，不调用应用激活。
4. 内容高度变化时合并 resize，避免流式 chunk 触发 frame thrash。

这是一项高平台风险：

- macOS 不使用简单的 Tauri 普通窗口冒充面板；实现前必须验证全屏 Space、点击后键盘输入和 Liquid Glass。
- Windows 先验证 `WS_EX_NOACTIVATE` 下点击、滚轮、文本编辑和关闭行为，再锁定实现方式。

### D7. OpenAI-compatible Provider

Provider 统一模型：

```text
ProviderConfig {
  id, label, kind,
  baseUrl, model, systemPrompt,
  extraHeaders?, enabled, isCustom
}
```

活动配置只有一个 Provider + Model。API Key 存在系统凭据库，按 `provider:<id>` 命名。请求使用 `POST <baseUrl>/chat/completions`：

首版预置包括 OpenAI、DeepSeek、OpenRouter、智谱 GLM、Kimi/Moonshot、硅基流动、Ollama 与 LM Studio；它们与自定义 OpenAI-compatible Provider 共用请求层，不引入厂商专有 SDK。

- `stream: true`
- `messages`: system prompt + user text
- `Authorization: Bearer <key>`，仅当 Provider 要求 API Key
- 可选额外 headers（OpenRouter attribution 等）

流解析同时支持 SSE `data:` 事件和 `[DONE]`，对非 SSE JSON 响应提供一次性回退。错误体先提取 Provider 的 JSON message，再归一化到认证、限流、超时、网络、空响应或协议错误。

**Alternatives considered**：

- 每个厂商独立 SDK：首版依赖和协议面过大。
- 直接复用 OpenAI 官方 SDK：不覆盖大量自定义和本地兼容端点，Rust 依赖也更重。

### D8. 流式会话与取消

Core 提供 `TranslationSession`，每个 UI surface 只允许一个 active session。每次启动分配单调递增 generation + cancellation token；所有 delta callback 在应用前比较 generation。新请求、语言/模型切换、关闭未固定浮窗和退出均取消旧请求。

前端收到：

- `state`: preparing / streaming / completed / cancelled / failed
- `delta`: 追加文本与 sessionId
- completion snapshot: 最终文本、语言方向、耗时、可用于历史的元数据

取消不写入历史，也不显示为失败。完成只在最终文本非空时写入历史。

### D9. 语言检测与路由

首版采用“本地轻量检测 + 手动覆盖”：

- 使用小型语言识别实现输出 source hint 与 confidence；脚本、字符集和常见语言对作为 hint。
- source 设置为 `auto` 时采用该 hint；低置信度把检测结果标记为不确定，但仍在 Prompt 中让模型按原文实际语言处理。
- target 为用户配置；若高置信度 source 与 target 相同，自动改用备用目标；低置信度不贸然换向。
- 浮窗与工作台显示实际方向，并允许手动覆盖。

UI 语言列表维护约 30 个稳定语言项；自定义语言通过独立输入保存为显示名，Prompt 使用该名称。核心不把 BCP-47 当作所有 Provider 的强制能力。

### D10. 设置、凭据与历史

```text
<app-data>/
  settings.json
  history.sqlite
  logs/
  updates/
```

桌面标识使用 `com.yiyu.desktop`。macOS 数据目录为 `~/Library/Application Support/Yiyu`，Windows 为 `%LOCALAPPDATA%\Yiyu`。允许 `YIYU_DATA_DIR` 覆盖以支持测试/便携验证。

- `settings.json`：原子写、版本号、`#[serde(default)]`、字段校验。保存失败不替换运行态。
- `history.sqlite`：WAL + `PRAGMA user_version` 顺序迁移，保存完成翻译；旧收藏标记保留以避免升级时自动删除旧记录。
- API keys：Keychain / Credential Manager，不写 `settings.json`、数据库、日志或更新包。

历史默认上限 500，允许用户在 10–10000 范围内配置；新记录按上限清理，旧收藏标记的记录暂不自动裁剪。明确确认“清空历史”会删除所有记录。单次输入默认上限 20000 字符，允许在 1000–200000 范围内配置。历史写入失败时当前结果仍保留，并明确提示未保存。

### D11. 网络与代理

Provider 请求与更新请求使用两套独立 transport：

- Provider：system / direct / manual；localhost 强制直连默认。
- Update：system / direct / manual；不读取或重建 Provider transport。

使用 `reqwest` + rustls，手动代理支持 HTTP/HTTPS/SOCKS5。系统代理探测使用经过 spike 验证的本地适配或 `proxy-watch`；代理会作为 transport 重建条件进入设置 revision。

### D12. 更新器

更新器采用自定义 GitHub Release 流程，而不是直接依赖 Tauri updater：

1. 请求 Release metadata，按 `target_os`、`target_arch` 和安装器类型选择唯一资产。
2. 记录版本、release notes、asset URL、SHA256SUMS URL。
3. 下载到应用数据目录的独立 updates 临时文件，报告双字节进度，支持取消。
4. 计算 SHA256 并对比清单；失败文件不能进入可安装状态。
5. macOS 只打开已验证 DMG，由用户完成覆盖；Windows 在用户确认后保存状态并启动已验证 NSIS。
6. 安装器启动失败保持当前进程运行，提供打开文件位置和重试。

**Rationale**：Tauri updater 不能完整覆盖 DMG 用户驱动的替换流程，也无法精确表达 bilidown 已验证的 Windows 失败恢复。独立更新器可以复用发布 manifest 和 SHA256 契约。

### D13. 视觉系统与前端性能

视觉方向按 `redesign-yiyu-frontend` 变更更新：

- 浅色/深色中性底面，适量青色强调，允许用户选择本地背景图片
- 半透明高斯模糊玻璃、柔和边界和按界面层级调整的圆角，保持文字可读
- 主窗口宽屏显示历史侧栏，窄窗口可通过明确入口展开历史
- macOS 与 Windows 分别验证窗口材质，不可用时使用可读回退
- 非必要动效遵循 `prefers-reduced-motion` 和应用设置

本 change 已勾选的前端任务记录首版基线，不代表后续重构已经完成；新方向的实施与验收由 `redesign-yiyu-frontend` 跟踪。

首屏策略沿用 bilidown：

- tokens/theme 在 React 挂载前应用，避免白闪。
- 主窗口首屏 only 加载 Queue → 改为 History + Workspace 基础组件。
- 设置页、历史详情、Provider 表单与更新流程按需动态加载。
- 空闲不轮询；语言/模型/设置由事件或显式动作触发。

### D14. 可测试性与 mock

`web` 提供与真实 backend 同契约的 `MockBackend`，支持：

- Provider 未配置、连接成功/失败、限流、流式中断、空结果
- 权限缺失、无选区、剪贴板为空
- 历史搜索、旧数据保留和清空
- 更新检查、下载、校验、安装失败

截图和交互 QA 使用 Playwright，不依赖真实 LLM。Rust core 使用单元测试覆盖 SSE、Prompt、语言路由、历史迁移、错误映射和剪贴板恢复状态机；平台壳对窗口/权限/代理/更新做单元 + 真机测试。

## Risks / Trade-offs

- **Tauri NSPanel/Windows non-activating 行为与 WebView 输入冲突** → 先做最小窗口 spike，再锁定平台适配；popup 只在使用时启用编辑焦点，失败时回退到标准浮窗并记录平台限制。
- **系统原生取词覆盖不完整** → 保留恢复式剪贴板回退、保守模式与可操作错误；真机矩阵覆盖浏览器、IDE、PDF、Electron、Office。
- **剪贴板恢复可能破坏复杂格式** → 使用独立快照适配并在用户外部复制时停止恢复；测试文本、富文本、图片和文件场景，无法保真的格式明确降级。
- **LLM 输出不遵守“只输出译文”** → 使用严格 Prompt + 响应边界处理；不静默进行额外的二次摘要，保留部分流式内容并允许重试。
- **小语言检测器短文本误判导致自动换向错误** → confidence 门槛 + 清晰语言方向 + 手动覆盖；低置信度不自动换向。
- **完整更新器在无签名阶段仍有安全风险** → SHA256 只证明资产一致性而非发布者身份；README 不宣称签名安全，正式签名后扩展发布校验。
- **React/Tauri 多窗口共享状态复杂** → 使用单向 event contract、revision patch 与 sessionId；所有窗口只读共享快照，不持有核心真相。
- **首版范围大，容易拖延到不可验证** → 按 tasks 分阶段 gate：基础设施、core 翻译、平台取词、工作台/历史、更新/发布；每阶段必须能通过对应 mock 与真机 gate。
