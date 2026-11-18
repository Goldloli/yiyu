# Spec Delta

## Purpose

定义译羽第一版的平台范围、应用标识、构建产物、版本一致性和发布质量门，使 macOS 与 Windows 安装包可重复构建并明确未签名发行限制。

## ADDED Requirements

### Requirement: 支持平台与系统基线
The first release SHALL support only macOS 13 or newer on Apple Silicon and Windows 10 22H2 or newer / Windows 11 on x64; Linux, Intel Mac and Windows ARM64 MUST be rejected as unsupported targets.

#### Scenario: 支持的 macOS
- **WHEN** 用户在运行 macOS 13+ 的 Apple Silicon Mac 上启动应用
- **THEN** 应用正常运行并显示平台对应菜单与窗口行为

#### Scenario: 不支持的架构
- **WHEN** 安装包运行在不支持的系统或处理器架构上
- **THEN** 安装或启动失败时提供明确的“不支持当前平台”说明

### Requirement: 应用标识与数据目录
The product SHALL expose the display name `译羽`, executable/product name `Yiyu` and identifier `com.yiyu.desktop`, and MUST use stable platform data locations separate from the installation directory.

#### Scenario: macOS 数据位置
- **WHEN** macOS 用户首次启动并保存设置
- **THEN** 配置、数据库和日志写入当前用户的应用支持目录，不写入安装包内部

#### Scenario: Windows 数据位置
- **WHEN** Windows 用户首次启动并保存设置
- **THEN** 配置、数据库和日志写入当前用户的 Local AppData 目录，不写入只读安装目录

### Requirement: 发布资产
Each release SHALL publish a macOS Apple Silicon DMG, a Windows x64 NSIS installer and one SHA256SUMS file containing both assets.

#### Scenario: 发布版本
- **WHEN** 维护者推送合法版本 tag
- **THEN** CI 生成对应平台的安装包与单一校验清单，并上传到同一 GitHub Release

#### Scenario: 校验发布资产
- **WHEN** 用户从发布页下载安装包
- **THEN** README 或发布说明提供使用 SHA256SUMS 校验文件的步骤

### Requirement: 版本一致性
The package manifest, Tauri configuration, Rust crate and frontend-visible version MUST remain identical before any release build.

#### Scenario: 版本不一致
- **WHEN** CI 检测到任一处产品版本不同
- **THEN** 构建失败并指出不一致的文件，不生成发布资产

### Requirement: 第一版签名状态
The first release MAY ship a macOS ad-hoc-signed and Windows unsigned installer, but documentation MUST state Gatekeeper and SmartScreen handling and MUST NOT claim formal signing or notarization.

#### Scenario: macOS 首次打开
- **WHEN** 用户安装未公证的 macOS 构建
- **THEN** README 与发布说明明确说明可能的系统拦截及安全放行步骤

#### Scenario: Windows 首次运行
- **WHEN** 用户运行未签名 Windows 安装器
- **THEN** README 与发布说明明确说明 SmartScreen 提示及“仍要运行”的范围

### Requirement: 开源与依赖许可
The repository SHALL use the MIT license for original source code, include third-party notices required by dependencies, and MUST NOT copy source code whose license conflicts with the selected distribution model.

#### Scenario: 发布源码
- **WHEN** 项目公开发布源码与安装包
- **THEN** 根许可证、第三方声明和依赖许可能够解释再分发条件

### Requirement: 跨平台质量门
Before release, CI SHALL run frontend typecheck and tests, Rust tests and clippy, OpenSpec strict validation, version consistency, and platform compile/build checks.

#### Scenario: 发布前检查失败
- **WHEN** 任一必需质量门失败
- **THEN** 发布工作流停止，不创建可下载版本或更新清单

### Requirement: 真机验收边界
CI SHALL be treated as compile and automated-test evidence only; macOS and Windows permission, capture, popup focus, proxy, installer and updater behavior MUST have platform-specific manual QA records.

#### Scenario: 缺少真机证据
- **WHEN** Windows 或 macOS 关键交互未被在对应系统验证
- **THEN** 发布状态保持未验收，不得仅以 CI 通过标记为完成
