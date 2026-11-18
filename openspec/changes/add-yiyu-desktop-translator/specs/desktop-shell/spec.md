# Spec Delta

## Purpose

定义译羽在 macOS 与 Windows 上的应用生命周期、系统入口、窗口职责和平台权限边界，保证后台常驻时轻量、可恢复且不会意外退出。

## ADDED Requirements

### Requirement: 首次启动与后台驻留
The application SHALL run as a single instance and show the main window by default after onboarding. Users SHALL be able to hide the main window on startup and independently enable system login startup.

#### Scenario: 首次启动
- **WHEN** 用户首次安装并启动译羽
- **THEN** 系统显示首次配置流程，并且只存在一个应用实例

#### Scenario: 后续启动
- **WHEN** 已完成配置的用户启动译羽
- **THEN** 应用显示主窗口，并在菜单栏或系统托盘提供入口

#### Scenario: 静默启动
- **WHEN** 已完成配置的用户启用“启动时隐藏主窗口”后启动译羽
- **THEN** 应用驻留菜单栏或系统托盘，不自动显示主窗口

#### Scenario: 开机启动
- **WHEN** 用户切换开机启动
- **THEN** 应用通过系统登录项更新该状态，并在设置中读取系统实际状态

### Requirement: 单实例唤起
The application SHALL route a second launch to the existing process, reveal the existing main window, and keep in-progress translation state.

#### Scenario: 重复启动
- **WHEN** 译羽已在运行且用户再次启动应用
- **THEN** 已有实例显示主窗口，当前翻译与历史状态保持可用；尚未完成首次配置时显示配置流程

#### Scenario: 从 macOS Dock 重新打开
- **WHEN** 译羽正在运行且没有可见窗口，用户点击 Dock 中的应用图标
- **THEN** 应用恢复主窗口；尚未完成首次配置时显示配置流程

### Requirement: 窗口职责与关闭语义
The desktop shell SHALL provide separate onboarding/settings, main workspace, and quick translation popup windows; closing the main window MUST hide it instead of terminating the application.

#### Scenario: 关闭主窗口
- **WHEN** 用户关闭主窗口
- **THEN** 主窗口隐藏到菜单栏或托盘，应用与后台快捷键继续运行

#### Scenario: 打开设置
- **WHEN** 用户从菜单栏、系统托盘或主窗口打开设置
- **THEN** 显示可独立关闭的设置窗口，关闭设置不会隐藏主窗口或退出应用

### Requirement: 菜单栏与托盘入口
The system entry SHALL expose selection translation, clipboard translation, open main window, settings, and quit; each platform MUST use its native menu conventions and terminology.

#### Scenario: macOS 菜单栏操作
- **WHEN** 用户在 macOS 菜单栏图标中选择操作
- **THEN** 菜单使用 macOS 可识别的应用、窗口与退出语义

#### Scenario: Windows 托盘操作
- **WHEN** 用户在 Windows 托盘图标中选择操作
- **THEN** “打开主窗口”可恢复主窗口，托盘菜单提供翻译、设置与退出

### Requirement: 可配置全局快捷键
The application SHALL register global shortcuts for selection translation, clipboard translation, and main-window opening; all shortcuts MUST be user-recordable, disposable, and conflict-aware.

#### Scenario: 使用默认快捷键
- **WHEN** 用户按下对应平台的默认划词快捷键
- **THEN** 应用开始读取当前选区并打开快捷翻译浮窗

#### Scenario: 快捷键冲突
- **WHEN** 用户录入已被系统或其他应用占用的组合键
- **THEN** 设置界面显示冲突错误，不保存无效快捷键，并继续保留上一组有效绑定

### Requirement: 权限状态可见
The application SHALL expose macOS Accessibility and any required Windows system capability status, provide a direct path to system settings, and MUST distinguish permission failure from empty selection.

#### Scenario: macOS 未授权
- **WHEN** 用户触发划词翻译但应用没有辅助功能权限
- **THEN** 浮窗或设置提示权限缺失并提供打开系统设置的入口

#### Scenario: 权限恢复
- **WHEN** 用户在系统设置中为当前版本授权并返回译羽
- **THEN** 应用重新检测系统对当前进程的授权；生效后允许再次触发翻译，尚未生效时保持未授权状态并提供恢复提示

### Requirement: 安全退出
The application SHALL exit only after user intent from the system entry or application menu; active translation requests MUST be cancelled and local state MUST be flushed before process termination.

#### Scenario: 菜单退出
- **WHEN** 用户明确选择退出译羽
- **THEN** 应用取消进行中的流式请求、保存待写入历史并正常结束进程

#### Scenario: 关闭窗口不等于退出
- **WHEN** 用户关闭任意普通窗口但没有选择退出
- **THEN** 应用进程保持运行且全局快捷键继续有效
