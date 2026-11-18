# 译羽 · Yiyu

一个专注的 macOS / Windows LLM 翻译工具。选中文字后按下快捷键，译羽会在光标旁出现，用你配置的 OpenAI-compatible 模型流式返回译文。

## 为什么是译羽

- 原生 Rust 核心，Tauri 2 外壳，安装体积和常驻开销尽量小。
- 不做通用 AI 工作台。主路径只有一件事：选中文字，看译文，继续阅读。
- 支持快捷键划词、剪贴板翻译和主窗口手动输入。
- Provider 由你选择。API Key 只保存在系统凭据库，不经过项目服务器。
- 翻译请求默认使用系统代理；localhost 模型服务默认直连。
- 历史记录保存在本机，保留上限由你设置；收藏不会被“清空历史”删除。

暂不包含截图 OCR、自动划词浮标、TTS、文档翻译、多模型并行对比和浏览器扩展。

## 支持平台

| 平台 | 架构 | 最低系统 |
| --- | --- | --- |
| macOS | Apple Silicon | macOS 13 |
| Windows | x64 | Windows 10 22H2 / Windows 11 |

当前不支持 Intel Mac、Windows ARM64 和 Linux。

## 功能

- 全局快捷键取词：macOS Accessibility 优先，Windows UI Automation 优先。
- 恢复式剪贴板回退：只在原生取词失败时使用，并尽量完整保存和恢复原剪贴板格式。
- OpenAI-compatible Provider：OpenAI、DeepSeek、OpenRouter、智谱 GLM、Kimi、硅基流动、Ollama、LM Studio 和自定义接口。
- 自动语言识别、默认目标语言、同语言自动换向，以及手动源/目标语言。
- SSE 流式结果、取消、重试和可读错误。
- 轻量非激活浮窗和“左侧历史 + 右侧工作台”主窗口。
- 本地 SQLite 历史、搜索、收藏、重译、删除和可配置保留上限。
- macOS Keychain / Windows Credential Manager 密钥存储。
- 简体中文与 English 界面，浅色/深色/跟随系统主题，减弱动效。
- 应用内更新检查、下载、取消、SHA256 校验和用户确认安装。

## 本地开发

环境要求：

- Node.js 20+
- pnpm 11
- Rust 1.90+
- macOS：Xcode Command Line Tools
- Windows：MSVC 工具链、Windows SDK 和 WebView2

```bash
pnpm install
pnpm dev
```

只查看前端 mock：

```bash
pnpm dev:web
```

浏览器访问 `http://127.0.0.1:1420/#/main`。开发 mock 不会调用真实模型，也不会读取系统选区。

## Provider 配置

1. 打开“设置 → 模型”。
2. 选择预置服务或添加自定义 Provider。
3. 输入 Base URL、API Key 和模型。
4. 点击“测试连接”，成功后设为当前使用。
5. 需要时填写自定义系统 Prompt。`{sourceLang}` 和 `{targetLang}` 会被替换为实际语言。

- Ollama 默认地址：`http://127.0.0.1:11434/v1`。
- LM Studio 默认地址：`http://127.0.0.1:1234/v1`。

## 网络与隐私

- 普通翻译请求与软件更新请求使用两套独立代理设置。
- 支持系统代理、直连、手动 HTTP / HTTPS / SOCKS5。
- API Key 不写入 `settings.json`、历史数据库、日志或前端存储。
- 默认日志只记录 Provider、模型、耗时、状态码等诊断信息，不记录原文或译文。
- 历史记录只在本机，永远不上传到项目方服务器。

## 数据目录

- macOS：`~/Library/Application Support/Yiyu`
- Windows：`%LOCALAPPDATA%\Yiyu`
- 测试/便携覆盖：`YIYU_DATA_DIR`

目录内包含：

```text
settings.json
history.sqlite
logs/
updates/
```

## 检查

```bash
actionlint
pnpm typecheck
pnpm test
pnpm qa:interactions
pnpm qa:screenshots
pnpm qa:screenshots:compare
pnpm build:web
node scripts/measure-first-load.mjs web/dist
pnpm release:check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
node scripts/version.mjs
openspec validate --all --strict
```

## 构建

```bash
pnpm build
pnpm tauri build --bundles dmg
pnpm tauri build --bundles nsis
```

Release 资产：

- `Yiyu_<version>_aarch64.dmg`
- `Yiyu_<version>_x64-setup.exe`
- `SHA256SUMS.txt`

## 签名与发布

第一版使用 macOS ad-hoc 签名、未公证，Windows 安装包未做 Authenticode 签名。

macOS 开发构建可能因 ad-hoc 签名变化而需要重新授权辅助功能；可使用无需 Apple 账号的本地开发签名，配置见 [macOS 签名说明](docs/macos-signing.md)。

- macOS 可能被 Gatekeeper 拦截：

```bash
sudo xattr -dr com.apple.quarantine /Applications/Yiyu.app
```

- Windows 可能出现 SmartScreen 未知发布者提示，可选择“更多信息 → 仍要运行”。

安装前请使用 Release 中的 `SHA256SUMS.txt` 校验下载文件。SHA256 校验能证明文件与发布清单一致，不替代正式代码签名。

## 项目结构

```text
crates/yiyu-core/   Rust 核心：Provider、SSE、语言、设置、历史
src-tauri/          Tauri 外壳：窗口、取词、托盘、快捷键、凭据、更新
web/                React 界面与浏览器 mock
scripts/            版本、性能与本地质量门
openspec/           产品规格和变更提案
docs/               架构、Provider、QA 与发布说明
```

架构细节见 [docs/architecture.md](docs/architecture.md)。
性能基线见 [docs/performance.md](docs/performance.md)。

## 协作规范

功能变更先创建 OpenSpec change，完成规格与任务并由维护者确认后再实现。分支、提交和发布约束见 [AGENTS.md](AGENTS.md)，贡献流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 许可

项目原创代码采用 [MIT License](LICENSE)。第三方依赖许可见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
