# Spec Delta

## Purpose

定义译羽从 GitHub Releases 获取、验证和安装新版本的可观察行为，确保更新流程可取消、可恢复，并且不会静默替换正在运行的应用。

## ADDED Requirements

### Requirement: 更新检查
The application SHALL check releases at most once per 24 hours when automatic checks are enabled, allow manual checks, and display the newest compatible version with release notes and asset availability.

#### Scenario: 自动检查
- **WHEN** 距上次成功检查超过 24 小时且用户启用自动检查
- **THEN** 应用在后台查询更新，不阻塞启动或翻译，也不在无更新时显示打扰性通知

#### Scenario: 手动检查
- **WHEN** 用户在设置中点击检查更新
- **THEN** 应用立即查询并显示检查中、无更新或可用版本状态

### Requirement: 平台资产精确匹配
The updater SHALL select only the asset matching the current operating system, architecture and installer type, and MUST reject ambiguous or mismatched release assets.

#### Scenario: macOS 更新
- **WHEN** 运行于 Apple Silicon macOS 的应用发现新版本
- **THEN** 只接受匹配的 aarch64 DMG 或 ZIP 资产

#### Scenario: Windows 更新
- **WHEN** 运行于 Windows x64 的应用发现新版本
- **THEN** 只接受匹配的 x64 NSIS 安装器

#### Scenario: 资产缺失
- **WHEN** 新版本没有当前平台的完整资产或校验清单
- **THEN** 应用显示安装包不可用，不下载错误文件

### Requirement: 下载进度与取消
The application SHALL report discrete download states and bounded progress, support cancellation, and preserve enough state to resume or retry an interrupted download safely.

#### Scenario: 下载进度
- **WHEN** 更新包正在下载
- **THEN** 界面显示下载字节、总字节、速度、预计剩余时间和可取消操作

#### Scenario: 用户取消
- **WHEN** 用户取消下载
- **THEN** 网络请求停止，未验证的部分文件不进入可安装状态，可重新开始或继续

### Requirement: SHA256 校验
The application MUST verify the downloaded installer against the release's SHA256SUMS before presenting any install action.

#### Scenario: 校验成功
- **WHEN** 安装包哈希与清单完全匹配
- **THEN** 应用进入“可安装”状态并显示安装按钮

#### Scenario: 校验失败
- **WHEN** 安装包哈希不匹配或清单不可读取
- **THEN** 应用删除或隔离无效文件，显示校验失败并允许重新下载

### Requirement: 用户确认安装
The application MUST NOT install or restart without explicit user action; macOS SHALL open the verified DMG for user-driven replacement and Windows SHALL launch the verified NSIS installer after saving local state.

#### Scenario: macOS 安装
- **WHEN** 用户在已校验的 macOS 更新上点击安装
- **THEN** 应用打开 DMG，用户自行完成覆盖安装，当前运行实例不被静默替换

#### Scenario: Windows 安装
- **WHEN** 用户在已校验的 Windows 更新上点击“安装并重启”
- **THEN** 应用先保存历史与设置，再启动安装器并退出；安装器启动失败时保持应用运行并显示错误

### Requirement: 更新失败可恢复
Update check, metadata, network, disk, checksum and installer-launch failures SHALL be readable and retryable without breaking translation, history or settings.

#### Scenario: 更新检查网络失败
- **WHEN** GitHub 请求超时或不可达
- **THEN** 应用显示网络错误和重试入口，现有翻译功能保持可用

### Requirement: 更新代理隔离
Software update requests SHALL use their own system, direct or manual proxy configuration and MUST NOT overwrite the Provider request proxy while changing update network settings.

#### Scenario: 切换更新代理
- **WHEN** 用户将更新代理改为手动 URL
- **THEN** 后续更新检查和下载使用该代理，LLM 请求继续使用原有 Provider 代理策略
