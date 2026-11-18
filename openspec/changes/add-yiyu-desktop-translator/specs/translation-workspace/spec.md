# Spec Delta

## Purpose

定义译羽的主要可观察界面，包括快捷翻译浮窗、主窗口翻译工作台、页面状态、交互动作和统一视觉语义，让划词翻译保持快速，让持续使用保持清晰。

## ADDED Requirements

### Requirement: 非激活快捷浮窗
The quick translation popup SHALL open near the invocation point without taking keyboard focus from the previously active application, remain within the current monitor work area, and size itself to available content without overlapping screen edges.

#### Scenario: 划词后显示浮窗
- **WHEN** 应用成功读取选区并开始翻译
- **THEN** 浮窗在光标或选区附近出现，原应用仍是输入焦点

#### Scenario: 屏幕边缘定位
- **WHEN** 触发点靠近显示器边缘或浮窗高度增长
- **THEN** 浮窗自动调整位置并保持在可见工作区内

### Requirement: 浮窗内容与动作
The quick popup SHALL show the actual language direction, active Provider/model, collapsible source text, streaming translation, copy, retry, pin and settings actions.

#### Scenario: 查看翻译结果
- **WHEN** 流式译文到达浮窗
- **THEN** 用户可滚动、折叠原文、复制译文、固定浮窗或对失败结果重试

#### Scenario: 复制译文
- **WHEN** 用户点击复制或按下浮窗复制快捷键
- **THEN** 关键内容写入剪贴板并显示短暂成功反馈，不修改历史中的原译文

### Requirement: 关闭与固定
The quick popup SHALL dismiss on outside click or Escape when unpinned, and MUST remain visible until explicitly closed when pinned.

#### Scenario: 未固定时点击外部
- **WHEN** 用户点击浮窗外部
- **THEN** 当前未固定的浮窗关闭并取消未完成请求

#### Scenario: 固定浮窗
- **WHEN** 用户点击固定按钮
- **THEN** 浮窗保持可见并允许用户继续滚动和复制，直到再次取消固定或关闭

#### Scenario: 错误自动取消固定
- **WHEN** 固定浮窗中的请求发生不可恢复错误
- **THEN** 浮窗解除固定并显示错误，避免错误结果被永久冻结在桌面

### Requirement: 主窗口翻译工作台
The main window SHALL provide an accessible history area and a translation workspace with source input, target language, active model, translate action and streamed output. A left history sidebar MAY collapse behind an explicit control when the available width is limited.

#### Scenario: 打开主窗口
- **WHEN** 用户从菜单栏或托盘打开主窗口
- **THEN** 显示可立即输入的翻译工作台，用户可通过左侧历史或明确入口搜索历史

#### Scenario: 主窗口手动翻译
- **WHEN** 用户输入文本并点击翻译或按下 `CmdOrCtrl+Enter`
- **THEN** 右侧译文区域以流式状态更新，完成后写入历史

### Requirement: 明确翻译状态
Each translation surface SHALL distinguish idle, preparing, streaming, completed, cancelled and failed states; stale or cancelled responses MUST NOT overwrite current content.

#### Scenario: 等待首个内容块
- **WHEN** 请求已发送但尚未收到首个增量
- **THEN** 界面显示进行中状态并允许取消，而不是展示空结果

#### Scenario: 请求被替换
- **WHEN** 用户改变语言、模型或发起新请求
- **THEN** 旧请求进入取消状态，只有最新请求可以更新当前译文

### Requirement: 统一的错误与重试反馈
The interface SHALL render normalized Provider and capture errors near the affected translation surface, preserve source text, and provide direct retry or settings actions when applicable.

#### Scenario: Provider 认证失败
- **WHEN** 请求因 API Key 无效而失败
- **THEN** 浮窗或工作台显示认证失败原因，并提供打开 Provider 设置的按钮

#### Scenario: 网络暂时失败
- **WHEN** 请求因超时或网络中断失败
- **THEN** 界面保留原文并显示重试按钮，重试只创建一次新请求

### Requirement: 视觉与动效规范
The product SHALL use the shared light/dark/system theme and restrained accent colors with legible glass or fallback surfaces, and SHALL provide a reduced-motion mode that suppresses nonessential transitions. A user-selected local background MAY appear behind the main workspace.

#### Scenario: 切换主题
- **WHEN** 用户选择浅色、深色或跟随系统
- **THEN** 所有窗口在无整窗闪烁的情况下应用对应色彩语义

#### Scenario: 减弱动效
- **WHEN** 用户启用减弱动态效果或系统要求减少动态
- **THEN** 浮窗与页面跳过非必要位移、缩放和透明度动画

### Requirement: 双语界面
The interface SHALL support Simplified Chinese and English, follow the system locale on first launch, default to Simplified Chinese when the locale is unsupported, and allow manual switching without losing current state.

#### Scenario: 首次加载语言
- **WHEN** 应用首次启动
- **THEN** 简体中文或 English 界面按系统语言自动选择

#### Scenario: 运行时切换语言
- **WHEN** 用户在设置中切换界面语言
- **THEN** 当前窗口与后续打开的窗口更新文案，未提交的输入和历史保持不变

### Requirement: 可访问交互
Interactive controls SHALL have accessible names, visible keyboard focus, readable contrast and non-color-only state cues; translation actions MUST remain operable at supported minimum window sizes.

#### Scenario: 键盘导航
- **WHEN** 用户仅使用键盘操作设置或主窗口
- **THEN** 可聚焦控件按合理顺序导航，焦点样式可见，关键操作可执行

#### Scenario: 最小窗口尺寸
- **WHEN** 主窗口缩放到支持的最小尺寸
- **THEN** 语言、模型、输入和结果操作仍可见或可滚动访问，不发生文本遮挡
