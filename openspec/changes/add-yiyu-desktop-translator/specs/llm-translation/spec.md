# Spec Delta

## Purpose

定义译羽的 LLM 翻译运行契约，包括请求协议、流式输出、语言方向、格式保留、取消重试和用户可理解的失败结果。

## ADDED Requirements

### Requirement: OpenAI-compatible 请求
The translation engine SHALL send requests to the configured OpenAI-compatible chat completion endpoint using the selected model, base URL, API key, and provider headers.

#### Scenario: 发送翻译请求
- **WHEN** 用户对有效文本发起翻译且当前 Provider 已配置
- **THEN** 应用向该 Provider 的 chat completion 端点发送包含系统指令、目标语言和原文的单次请求

#### Scenario: 缺少配置
- **WHEN** 当前 Provider 缺少 Base URL、模型或 API Key
- **THEN** 应用不发起网络请求，并显示缺失字段和前往设置的入口

### Requirement: 增量流式结果
The translation engine SHALL expose model output incrementally while the response is being received, and MUST keep partial text visible when a later chunk fails or the request is cancelled.

#### Scenario: 流式输出
- **WHEN** Provider 返回增量内容块
- **THEN** 浮窗或主窗口按接收顺序更新译文，不等待完整响应

#### Scenario: 流式中途失败
- **WHEN** 已收到部分内容后连接中断或 Provider 返回错误
- **THEN** 应用保留部分译文，标记为未完成，并提供重试

### Requirement: 专注翻译输出
The default translation instruction SHALL request only the translated result without explanations, commentary, added facts, summaries or multiple alternatives.

#### Scenario: 普通文本翻译
- **WHEN** 用户提交一段普通文本
- **THEN** 结果只包含译文，不附带“以下是翻译”等前后说明

### Requirement: 格式与代码保留
The translation instruction SHALL preserve paragraphs, Markdown structure, lists, code blocks, indentation, placeholders, URLs and inline markup unless changing them is required for the target language.

#### Scenario: Markdown 与代码
- **WHEN** 输入包含 Markdown 标题、列表和代码块
- **THEN** 输出保持对应结构和代码内容，只翻译自然语言部分

### Requirement: 自动语言方向
The application SHALL support automatic source-language detection, a user-configured target language, and automatic direction reversal when the detected source already matches the target language.

#### Scenario: 源语言不同于目标
- **WHEN** 检测到的源语言与默认目标语言不同
- **THEN** 应用按“源语言 → 默认目标语言”翻译

#### Scenario: 源语言等于目标
- **WHEN** 检测到的源语言与默认目标语言相同
- **THEN** 应用自动改用用户配置的备用目标语言，并在浮窗中显示实际语言方向

#### Scenario: 手动覆盖
- **WHEN** 用户手动选择源语言或目标语言
- **THEN** 应用使用手动选择，直到用户清除该选择或关闭当前会话

### Requirement: 自定义 Prompt
The application SHALL allow each Provider to persist a custom system prompt with target-language and source-language placeholders, and MUST fall back to a built-in translation prompt when the custom value is empty.

#### Scenario: 自定义 Prompt
- **WHEN** 用户为当前 Provider 保存合法 Prompt 并发起翻译
- **THEN** 实际请求使用该 Prompt，并正确替换语言占位符

#### Scenario: Prompt 为空
- **WHEN** 用户清空 Provider 的自定义 Prompt
- **THEN** 应用使用内置专注翻译 Prompt，并保持原有输出约束

### Requirement: 请求取消与最新请求优先
Each translation surface SHALL accept one active model request at a time; a new translation, language change or dismissal MUST cancel the superseded request and prevent late chunks from replacing current state.

#### Scenario: 快速连续翻译
- **WHEN** 第一条请求尚未完成时用户发起新的划词翻译
- **THEN** 旧请求被取消，新请求成为唯一可更新界面结果的活动请求

#### Scenario: 关闭浮窗
- **WHEN** 用户在流式翻译期间关闭未固定的浮窗
- **THEN** 当前网络请求停止，已取消请求不得重新打开浮窗

### Requirement: 可重试错误归一化
The application SHALL normalize authentication, rate limit, timeout, network, invalid response, empty result, cancellation and Provider API errors into readable messages with a safe retry action where applicable.

#### Scenario: 限流错误
- **WHEN** Provider 返回限流响应
- **THEN** 应用显示限流原因、建议稍后重试，并保留原文以便直接重译

#### Scenario: 取消不显示为失败
- **WHEN** 用户主动关闭或替换请求
- **THEN** 界面将其视为取消状态，不显示错误横幅或失败历史

### Requirement: 翻译文本隐私
Translation source text, translated text, API keys and request headers MUST NOT be written to diagnostic logs by default; diagnostics MAY record provider ID, model, timing, status and redacted error metadata.

#### Scenario: 请求失败诊断
- **WHEN** 用户查看失败请求的诊断信息
- **THEN** 日志包含 Provider、模型、耗时和状态码，但不包含 API Key 或完整原文
