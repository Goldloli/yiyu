# Spec Delta

## Purpose

定义译羽本地翻译历史的持久化、保留、检索和清理行为，使用户可以快速找回译文，同时明确数据只保存在当前设备并可被用户完全控制。

## ADDED Requirements

### Requirement: 成功翻译自动保存
When auto-save is enabled, the application SHALL persist each successfully completed translation with source text, translated text, detected source language, target language, Provider, model, timestamp and duration.

#### Scenario: 翻译完成
- **WHEN** 流式响应正常完成且存在非空译文
- **THEN** 应用将一条完成记录写入本地历史，并在主窗口左侧历史中可见

#### Scenario: 失败或取消
- **WHEN** 请求失败、被取消或被新请求替换
- **THEN** 应用不新增完成历史，已存在的部分流式文本不写入持久历史

### Requirement: 自动保存开关
The application SHALL allow users to disable automatic history saving without disabling current-session translation or existing history.

#### Scenario: 关闭自动保存
- **WHEN** 用户关闭自动保存后完成新的翻译
- **THEN** 当前结果仍显示，但不会新增普通历史记录

### Requirement: 用户可配置保留上限
The application SHALL allow users to choose the history retention limit, defaulting to 500 and enforcing a bounded valid range with visible validation. Previously protected records MAY be exempt from automatic pruning during migration.

#### Scenario: 降低历史上限
- **WHEN** 用户把上限从较大值改为较小值
- **THEN** 应用按创建时间删除超出上限的普通记录，并保留旧版本已保护的记录

#### Scenario: 输入非法上限
- **WHEN** 用户提交超出允许范围的保留上限
- **THEN** 设置不生效，原上限保持不变并显示有效范围

### Requirement: 旧数据安全迁移
The application SHALL remove the favorites feature without deleting previously saved records. Legacy favorite markers MAY be retained internally to prevent automatic deletion of older records.

#### Scenario: 已有收藏记录
- **WHEN** 用户从旧版本升级
- **THEN** 所有旧收藏继续作为普通历史显示，升级本身不删除记录

#### Scenario: 清空历史
- **WHEN** 用户确认清空所有历史
- **THEN** 所有历史记录均被删除，包括旧收藏记录

### Requirement: 历史检索与操作
The history view SHALL support keyword search over source and translated text, and MUST allow copying, retranslating, deleting one entry and clearing all history with confirmation. It SHALL NOT expose favorite controls.

#### Scenario: 搜索历史
- **WHEN** 用户在历史搜索框输入关键词
- **THEN** 列表只显示原文、译文、Provider 或模型匹配的记录，并标明无结果状态

#### Scenario: 重译历史
- **WHEN** 用户从历史条目选择重译
- **THEN** 应用把原文装入当前工作区并使用当前活动 Provider 与模型发起新请求

### Requirement: 本地数据边界
History SHALL remain on the current device, MUST NOT be uploaded to a project-operated service, and exported or diagnostic features MUST omit API keys and redact unrelated credentials.

#### Scenario: 查看历史
- **WHEN** 用户在无网络环境打开历史
- **THEN** 已保存的本地记录仍可浏览、搜索和复制

### Requirement: 持久化失败可见
The application SHALL surface history write, deletion and migration failures with a readable message and retry path; it MUST NOT claim a record was saved or removed when the local transaction failed.

#### Scenario: 历史写入失败
- **WHEN** 翻译完成但本地写入失败
- **THEN** 当前译文保持显示，应用明确提示历史未保存并允许重试写入
