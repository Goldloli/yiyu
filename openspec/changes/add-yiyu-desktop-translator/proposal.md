# Proposal

## Why

现有轻量翻译工具通常依赖传统机器翻译接口，复杂语境、术语和格式保留能力有限；功能完整的桌面翻译产品又往往体积大、交互重。译羽需要以 Rust + Tauri 构建一个启动快、常驻轻、界面克制的 macOS 与 Windows 桌面翻译工具，让用户选中文字后立刻获得基于自有 LLM Provider 的流式译文。

## What Changes

- 新建 `Yiyu` / `译羽` 桌面应用，采用 Rust workspace core、Tauri 2 shell 和 React 前端。
- 提供三种翻译入口：全局快捷键读取当前选区、系统剪贴板翻译、主窗口手动输入。
- 采用原生无障碍取词优先、恢复式剪贴板模拟回退的选区捕获链路；默认启用回退，并提供保守模式。
- 统一接入 OpenAI-compatible LLM Provider，支持预置服务、自定义 Base URL、API Key、模型、系统 Prompt、连接测试和模型列表获取。
- 翻译请求使用 SSE 流式输出，自动识别源语言，自动处理目标语言与同语言换向。
- 提供轻量的非激活快捷浮窗，以及“左侧历史 + 右侧翻译工作台”的主窗口。
- 本地保存成功翻译历史，支持用户配置上限、搜索、复制、重译、删除和清空。
- API Key 使用 macOS Keychain 与 Windows Credential Manager 保存；普通配置文件不包含明文凭据。
- 网络默认跟随系统代理，localhost 默认直连，并支持直连与手动 HTTP/HTTPS/SOCKS5 代理。
- 提供完整应用内更新：检查、下载、SHA256 校验、用户确认安装、失败恢复。
- 提供简体中文与 English 双语文案、浅色/深色/跟随系统主题和减弱动态效果。
- 提供 macOS Apple Silicon 与 Windows x64 发布配置；第一版不提供 Intel Mac、Windows ARM64 或 Linux。

### Out of Scope

- 截图 OCR、自动划词浮标、朗读/TTS。
- 多模型并行翻译与结果对比。
- 总结、解释、润色、改写等通用 AI 文本任务。
- Anthropic、Gemini 等原生协议，以及内置共享 API Key、免费代理或本地打包模型。
- 正式 Apple Developer ID 公证与 Windows Authenticode 商业签名；第一版使用 ad-hoc / 未签名构建，但保留后续签名配置入口。
- 文档翻译、字幕翻译、PDF 翻译、浏览器扩展和移动端。

## Capabilities

### New Capabilities

- `desktop-shell`: 应用生命周期、菜单栏/托盘、主窗口与设置窗口、单实例、全局快捷键、系统权限、平台差异和退出语义。
- `text-capture`: 从其他应用捕获选区、剪贴板取词回退、保守模式、剪贴板恢复和输入边界。
- `llm-translation`: OpenAI-compatible Provider、流式翻译、语言方向、Prompt、超时取消、代理和错误归一化。
- `provider-settings`: Provider 配置、凭据安全存储、模型发现、连接测试、配置迁移与网络设置。
- `translation-workspace`: 快捷浮窗、主窗口工作台、页面状态、主题与视觉规范、复制/重译/固定等可观察交互。
- `translation-history`: 成功翻译的本地历史、检索、用户可配置保留上限、删除与清空。
- `app-update`: 更新检查、下载、校验、安装确认、取消、恢复和发布资产匹配。
- `platform-distribution`: macOS/Windows 构建、安装包、版本一致性、CI、未签名发行说明和发布资产。

### Modified Capabilities

无。

## Impact

- 新建 `crates/yiyu-core`、`src-tauri`、`web`、`scripts` 和 `.github/workflows`。
- 新增 Tauri 2、React 19、TypeScript、Vite、Tailwind CSS、Zustand、SQLite、reqwest、keyring、全局快捷键与平台无障碍依赖。
- 新增 macOS Accessibility 权限、Windows UI Automation、剪贴板模拟与相关真机验收。
- 新增 Provider API Key、系统代理、应用数据目录、更新检查和 GitHub Release 获取等外部交互。
- 新增 macOS `.dmg` 与 Windows NSIS 发布资产，以及 SHA256 校验文件。
- 初始仓库目前只有工具链配置，无业务代码；本 change 同时定义产品基线、规格和实现顺序。
