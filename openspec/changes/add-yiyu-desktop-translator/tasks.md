# Tasks

## 1. 工程基线与契约

- [x] 1.1 创建 workspace、`crates/yiyu-core`、`src-tauri`、`web`、脚本与基础配置；以 `cargo check --workspace`、`pnpm install`、`pnpm build:web` 验证目录和依赖可用
- [x] 1.2 配置 MIT LICENSE、第三方许可清单、`productName: Yiyu`、bundle identifier、macOS 13 / Windows 10 22H2 基线；以构建配置读取和版本脚本输验证
- [x] 1.3 实现统一版本同步脚本，覆盖 package manifest、Tauri config、Rust crate 与前端版本常量；以 `node scripts/version.mjs` 检查不一致时失败
- [x] 1.4 建立 typed IPC contract、DTO adapter 和 MockBackend 场景集；以 `pnpm typecheck` 和契约单测验证命令名、字段和事件载荷
- [x] 1.5 建立前端 Vitest、Rust test、clippy、OpenSpec strict 与基础 CI workflow；以本地等价命令和一次 CI 配置 lint 验证质量门

## 2. Core 设置、历史与凭据

- [x] 2.1 实现 `AppPaths`、设置 schema、默认值、字段校验与原子 JSON 写入；以临时目录测试验证损坏/写入失败不替换运行态
- [x] 2.2 实现 `settings` revision 与 patch 合并，处理并发旧响应；以 revision 单测验证旧 patch 不得覆盖新值
- [x] 2.3 实现 Keychain / Credential Manager 适配接口、Provider 凭据引用与删除清理；以平台条件编译测试验证配置文件和日志无明文 Key
- [x] 2.4 实现日志初始化与敏感信息 redaction 策略；以快照测试验证 API Key、Authorization 和原文不会进入默认日志
- [x] 2.5 建立 history SQLite WAL schema、迁移和普通/收藏 CRUD；以内存与临时库测试验证迁移、上限清理和收藏保留
- [x] 2.6 实现历史自动保存开关、用户可配置上限和写入失败结果；以 repository 测试验证完成才写入、失败可重试
- [x] 2.7 实现系统代理、直连、手动 HTTP/HTTPS/SOCKS5 与 localhost bypass 的独立 Provider transport；以代理解析和请求构造测试验证策略隔离

## 3. LLM 与翻译核心

- [x] 3.1 定义 ProviderConfig、预置 Provider、自定义 Provider 与单一活动模型状态；以 serde 往返和自定义 Provider 单测验证
- [x] 3.2 实现 OpenAI-compatible chat completion 请求构造、Prompt 占位符替换和认证头；以不同 Provider 请求快照验证 URL、模型、消息和 headers
- [x] 3.3 实现 SSE `data`/`[DONE]` 解析、非 SSE JSON 回退和 malformed chunk 容错；以 fixture 驱动的流解析测试验证增量和错误体
- [x] 3.4 实现 `TranslationSession` generation、CancellationToken、delta 回调和完成/取消/失败状态机；以并发单测验证旧请求不能覆盖新请求
- [x] 3.5 实现内置专注翻译 Prompt、Markdown/代码保留指令和自定义 Prompt 恢复默认；以 Prompt 单测验证语言占位符与空值回退
- [x] 3.6 实现轻量源语言检测、目标语言解析、低置信度保护和同语言自动换向；以语言路由表驱动测试验证 manual override 与备用目标
- [x] 3.7 建立约 30 个常用语言与自定义目标语言的数据模型、显示名和选择约束；以可供性测试验证每个语言项可序列化并进入 Prompt
- [x] 3.8 实现连接测试、模型列表获取、超时、上限、限流和网络错误归一化；以 mock HTTP server 测试验证认证/限流/超时/空结果

## 4. Tauri 外壳与选区捕获

- [x] 4.1 配置单实例、主窗口/设置/onboarding 隐藏或按需显示、菜单栏/托盘、窗口状态和安全退出；以 shell 单测 + mock window label 验证启动不弹主窗口
- [x] 4.2 编写最小 Tauri capabilities，授予四条窗口路由、事件、剪贴板、通知与 opener 所需权限；以 capability 单测验证 popup/settings 关键 API 被授权
- [ ] 4.3 实现全局快捷键平台默认值、录制、清空、冲突检测与持久化；以 shortcut 单测和真机 smoke 验证快捷键不重复注册
- [ ] 4.4 实现 macOS Accessibility 权限检测、聚焦元素读取和系统设置跳转；以真实系统 smoke 验证授权前/后状态与错误分类
- [ ] 4.5 实现 Windows UI Automation 聚焦元素与文本模式读取；以 Windows 真机 smoke 验证浏览器、记事本和 VS Code 场景
- [ ] 4.6 实现剪贴板快照/模拟复制/变化检测/恢复状态机及 conservative mode；以单测和平台真机验证外部复制不被覆盖
- [ ] 4.7 完成 macOS NSPanel spike，验证非激活显示、点击后键盘输入、全屏 Space、Liquid Glass 与关闭；以最小 sample 和真机 QA 记录决定插件或自研桥接
- [ ] 4.8 完成 Windows non-activating popup spike，验证 `WS_EX_NOACTIVATE`、滚动、编辑和关闭；以窗口 sample 和真机 QA 记录锁定扩展样式
- [ ] 4.9 接入浮窗定位、工作区 clamp、内容高度 resize、pin、outside-click 与 Escape；以单元布局测试和双平台截图验证边界
- [ ] 4.10 实现菜单/托盘动作、关闭主窗口隐藏、设置窗口相对定位与退出保护；以窗口生命周期真机和自动化测试验证无不可恢复隐藏状态

## 5. 前端体验

- [x] 5.1 实现羽光青蓝 tokens、浅/深/系统主题、减弱动效和首帧主题应用；以视觉快照和 `prefers-reduced-motion` 测试验证无白闪
- [x] 5.2 实现简体中文/English 双语资源、系统语言解析与运行时切换；以 i18n 单测验证缺失 key fallback 和状态保留
- [x] 5.3 实现 Button、IconButton、Select、SegmentedControl、Switch、TextArea、StreamText、Toast、Tooltip 等基础组件；以组件测试和截图画廊验证稳定尺寸与键盘焦点
- [x] 5.4 实现首次配置与 Provider 设置页，包括预置、自定义、Key、连接测试、模型发现、Prompt 与代理；以 MockBackend 交互测试验证失败草稿保留
- [x] 5.5 实现快捷浮窗，包括语言方向、模型名、原文折叠、流式译文、复制、重译、pin 与设置入口；以 Playwright 桌面/窄屏截图和交互测试验证
- [x] 5.6 实现主窗口左侧历史 + 右侧双栏翻译工作台，包括手动输入、`CmdOrCtrl+Enter`、语言/模型选择和空状态；以布局、最小尺寸和键盘测试验证
- [x] 5.7 实现翻译 session 事件订阅、流式追加、取消、latest-wins 和部分失败保留；以 mock event 单测验证旧 session 不更新新界面
- [x] 5.8 实现历史搜索、收藏、复制、重译、删除、清空以及自动保存/上限设置；以 Memory/Mock history 测试验证收藏不被清空
- [x] 5.9 建立浏览器 mock/gallery 和 Playwright 截图基线，覆盖中文/English、浅/深色、主窗口、浮窗、设置和错误状态；以 CI 截图差异和交互脚本验证

## 6. 应用内更新

- [x] 6.1 实现 Release metadata 获取、版本比较和 macOS aarch64 DMG / Windows x64 NSIS 精确资产匹配；以 fixture 测试验证缺失、歧义和错误架构被拒绝
- [x] 6.2 实现更新专用 system/direct/manual 代理与 Provider transport 隔离；以设置切换测试验证两个 transport 互不覆盖
- [x] 6.3 实现更新下载流、进度节流、取消、断点和临时文件生命周期；以 mock download server 验证进度、取消和重试
- [x] 6.4 实现 SHA256SUMS 下载、校验和无效文件隔离；以哈希 fixture 验证成功和篡改失败
- [ ] 6.5 实现 macOS 打开已验证 DMG、Windows 保存状态后启动 NSIS 与失败回退；以命令构造单测和真机 smoke 验证不静默替换
- [x] 6.6 实现设置/关于页更新 UI、状态事件、发布说明与失败重试；以 MockBackend 交互测试验证检查、下载、校验、取消和安装状态

## 7. 发布与文档

- [ ] 7.1 配置 macOS aarch64 DMG、Windows x64 NSIS、图标、bundle metadata 和数据目录；以双平台 bundle 构建和安装 smoke 验证
- [x] 7.2 建立 CI matrix，覆盖前端类型/测试、Rust 测试/clippy、OpenSpec strict、版本一致性和双平台 compile；以 CI 本地复跑和 GitHub workflow 语法验证
- [x] 7.3 建立 release workflow，上传 macOS DMG、Windows NSIS 和唯一 SHA256SUMS；以本地 dry-run/资产路径脚本验证名称和单一校验清单
- [x] 7.4 编写 README、架构说明、Provider 配置、代理、隐私、未签名 Gatekeeper/SmartScreen 说明和贡献指南；以文档命令与链接检查验证
- [x] 7.5 编写 `docs/qa/` 双平台真机验收矩阵，覆盖权限、浏览器/IDE/PDF/Office 取词、浮窗 focus、剪贴板恢复、代理、安装与更新；以记录模板和至少一次 macOS smoke 验证流程可执行
- [x] 7.6 建立首屏 JS 预算、空闲 CPU/RSS 和唤醒性能测量脚本；以基线报告验证没有空闲轮询且首屏不被设置/更新模块阻塞

## 8. 集成与发布门

- [ ] 8.1 端到端验证划词、剪贴板、手动三类入口在同一 Provider 下的流式、取消、失败和历史闭环；以 Mock + macOS 真机脚本验证
- [x] 8.2 端到端验证 Provider 未配置、认证失败、限流、代理失败、空结果与恢复路径；以故障注入测试验证原文和目标配置不丢失
- [ ] 8.3 端到端验证主窗口关闭隐藏、单实例唤起、退出保存、设置关闭和托盘动作；以双平台真机检查表验证
- [x] 8.4 执行发布前质量门 `pnpm typecheck && pnpm test && pnpm build:web && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && openspec validate --all --strict`；记录输出与未执行真机项
- [ ] 8.5 在 macOS Apple Silicon 与 Windows x64 真机完成安装、首次配置、连续 2 小时后台驻留、更新检查/下载/校验/安装 smoke；将证据写入 `docs/qa/`，未验证项不得勾选

## Workflow follow-up

- change 文档经用户确认后再进入 apply 阶段；实现期间每个阶段完成后同步 CodeGraph 索引并更新 Beads 状态。
- 全部任务与真机验收完成后，运行 OpenSpec archive 流程并复制 delta capabilities 到主 specs。
