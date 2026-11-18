# Spec Delta

## Purpose

定义译羽的 Provider 配置、凭据安全、模型发现和网络策略，使 BYOK 用户能够安全地配置任意 OpenAI-compatible 服务并快速定位连接问题。

## ADDED Requirements

### Requirement: Provider 预置与自定义
The application SHALL include editable presets for common OpenAI-compatible services and allow users to create custom Providers with a display name, base URL, model, API key and system prompt.

#### Scenario: 使用预置服务
- **WHEN** 用户在首次配置中选择预置 Provider
- **THEN** 表单预填该服务的 Base URL 与建议模型，用户仍可修改全部字段

#### Scenario: 创建自定义 Provider
- **WHEN** 用户添加自定义 OpenAI-compatible Provider
- **THEN** 应用保存该配置并在模型选择列表中将其标记为自定义

### Requirement: 单一活动模型
The application MAY store multiple Provider 配置，但同一时刻 SHALL 只有一个活动 Provider 和一个活动模型用于翻译。

#### Scenario: 切换活动模型
- **WHEN** 用户从模型选择器中选择另一个 Provider 或模型
- **THEN** 后续翻译使用新选择，已完成的流式请求不被改写，新请求使用新模型

### Requirement: API Key 系统安全存储
The application SHALL store API keys in macOS Keychain or Windows Credential Manager and MUST NOT persist plaintext keys in settings, history, logs, exports or renderer storage.

#### Scenario: 保存密钥
- **WHEN** 用户保存 Provider API Key
- **THEN** 密钥写入系统凭据库，配置文件只记录凭据引用和 Provider 标识

#### Scenario: 删除 Provider
- **WHEN** 用户删除自定义 Provider
- **THEN** 应用同时删除对应系统凭据与非敏感配置，不留下可复用密钥

### Requirement: 连接测试
The provider settings SHALL let users test the current base URL, API key, model and proxy without saving an invalid configuration as active, and MUST show latency or a readable failure reason.

#### Scenario: 连接成功
- **WHEN** 用户使用有效配置执行连接测试
- **THEN** 应用显示成功状态和可测延迟，并允许用户提交为活动配置

#### Scenario: 连接失败
- **WHEN** 连接测试遇到认证、网络、代理或模型错误
- **THEN** 应用显示对应原因，保留未保存的编辑内容供用户修正

### Requirement: 模型发现与手动回退
The application SHALL attempt to fetch available chat models from the Provider's model-list endpoint and MUST allow users to enter a model identifier manually when discovery is unsupported or fails.

#### Scenario: 获取模型列表
- **WHEN** Provider 支持模型列表且认证有效
- **THEN** 设置页显示模型选项并排除明显非对话模型

#### Scenario: 模型列表不可用
- **WHEN** Provider 不实现模型列表或请求失败
- **THEN** 用户仍可手动填写模型标识并保存或测试

### Requirement: 原子设置持久化
Provider 与通用设置 SHALL validate before commit, expose field-level errors, write atomically, and preserve the last valid runtime configuration when persistence fails.

#### Scenario: 设置保存失败
- **WHEN** 本地磁盘写入失败
- **THEN** 当前运行配置不变，界面保留草稿并显示可重试的保存错误

#### Scenario: 旧配置迁移
- **WHEN** 应用升级后读取到旧版本设置结构
- **THEN** 迁移保留可识别字段并为新增字段提供安全默认值

### Requirement: LLM 请求代理
The application SHALL default to the system proxy, bypass proxy for localhost endpoints, and allow users to choose direct or manual HTTP, HTTPS or SOCKS5 proxy for Provider requests.

#### Scenario: 本地模型服务
- **WHEN** Provider Base URL 指向 `localhost`、`127.0.0.1` 或 `::1`
- **THEN** 即使启用了系统代理，本地请求仍默认直连

#### Scenario: 手动代理无效
- **WHEN** 用户输入格式不受支持的手动代理 URL
- **THEN** 设置页阻止提交活动配置并显示代理格式错误

### Requirement: 不提供共享凭据
The release build MUST NOT contain a project-owned shared API key, free translation proxy, or hidden fallback Provider that sends text without user configuration.

#### Scenario: 无活动 Provider
- **WHEN** 用户尚未配置任何有效 Provider
- **THEN** 翻译入口提示先配置模型，任何原文都不会被发送到第三方服务

### Requirement: 自定义 Prompt 持久化
Provider settings SHALL persist the custom system prompt independently from the API key, validate supported language placeholders, and provide a way to restore the built-in prompt.

#### Scenario: 恢复内置 Prompt
- **WHEN** 用户选择恢复默认 Prompt
- **THEN** 应用清除该 Provider 的自定义 Prompt 并在后续请求使用内置翻译指令
