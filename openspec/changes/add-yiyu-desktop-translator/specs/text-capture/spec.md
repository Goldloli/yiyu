# Spec Delta

## Purpose

定义译羽如何从其他应用获取待翻译文本，确保快捷键划词在常见应用中可用，同时避免不可见地破坏用户剪贴板或产生误导性的空结果。

## ADDED Requirements

### Requirement: 原生选区读取优先
The application SHALL first attempt to read the current text selection through the operating system accessibility interface before using clipboard simulation.

#### Scenario: 原生读取成功
- **WHEN** 当前焦点应用允许通过辅助功能或 UI Automation 读取选区
- **THEN** 应用直接取得选中文本并开始翻译，不触发复制快捷键

### Requirement: 恢复式剪贴板回退
The application MAY simulate the platform copy shortcut when native selection reading fails, but it MUST preserve the user's prior clipboard content and MUST NOT restore over a newer user-initiated clipboard change.

#### Scenario: 回退读取成功
- **WHEN** 原生读取失败且模拟复制成功取得选中文本
- **THEN** 应用返回选中文本、恢复可保留的原剪贴板内容，并打开翻译流程

#### Scenario: 用户同时复制
- **WHEN** 回退操作进行期间用户主动复制了新的内容
- **THEN** 应用检测到外部剪贴板变化，不覆盖用户的新内容

### Requirement: 保守取词模式
The application SHALL provide a conservative capture mode that disables clipboard simulation and uses only native accessibility reading.

#### Scenario: 保守模式下原生读取失败
- **WHEN** 用户启用保守取词模式且辅助功能无法读取选区
- **THEN** 应用显示取词失败说明，不模拟复制、不修改剪贴板

### Requirement: 无选区与权限失败区分
The application SHALL distinguish empty selection, unsupported target application, missing system permission, cancelled capture, and clipboard permission or access failure with actionable messages.

#### Scenario: 没有选中文本
- **WHEN** 用户触发划词翻译但当前焦点位置没有文本选区
- **THEN** 应用提示“未检测到选中文本”，并提供手动输入入口

#### Scenario: 目标应用不支持原生取词
- **WHEN** 当前应用不暴露可读取的文本选区且回退也未取得文本
- **THEN** 应用说明当前应用无法自动取词，并建议使用剪贴板或手动输入

### Requirement: 剪贴板翻译不写入剪贴板
The application SHALL translate clipboard text without simulating another copy or modifying the clipboard contents.

#### Scenario: 剪贴板中有文本
- **WHEN** 用户触发剪贴板翻译且剪贴板包含非空文本
- **THEN** 应用直接读取该文本并开始翻译，剪贴板内容保持不变

#### Scenario: 剪贴板为空或不含文本
- **WHEN** 剪贴板为空或只包含不支持的内容
- **THEN** 应用显示可读提示并允许用户改用手动输入

### Requirement: 输入规范化与空内容保护
The application SHALL trim only non-content edge whitespace, preserve internal line breaks, Markdown, code indentation and Unicode text, and MUST reject empty input before sending a model request.

#### Scenario: 保留格式
- **WHEN** 输入包含 Markdown、列表或代码缩进
- **THEN** 发送给支持格式保留的 LLM 的内容保留原有结构

#### Scenario: 空输入
- **WHEN** 捕获或手动输入的内容只有空白字符
- **THEN** 应用不创建模型请求，并显示空内容提示

### Requirement: 单次输入边界
The application SHALL enforce a configurable maximum input length and MUST provide a readable error when the current text exceeds that limit.

#### Scenario: 输入超过上限
- **WHEN** 用户提交的文本超过当前输入上限
- **THEN** 应用拒绝发起请求，保留已输入文本并显示当前上限和可调整入口
