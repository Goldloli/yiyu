# Provider Setup

译羽第一版只实现 OpenAI-compatible chat completions 协议。

## Request shape

```text
POST <baseUrl>/chat/completions

{
  "model": "<model>",
  "stream": true,
  "messages": [
    { "role": "system", "content": "<translation prompt>" },
    { "role": "user", "content": "<source text>" }
  ]
}
```

模型列表使用：

```text
GET <baseUrl>/models
```

如果服务不实现模型列表，翻译仍然可以通过手动输入模型名使用。

## Local services

- Ollama: `http://127.0.0.1:11434/v1`
- LM Studio: `http://127.0.0.1:1234/v1`

localhost / `127.0.0.1` / `::1` 默认绕过代理。

## Secrets

- API Key 保存在 macOS Keychain 或 Windows Credential Manager。
- Renderer 只能提交新 Key、删除 Key，不能读取已保存 Key。
- 测试连接和模型获取由 Rust 后端读取凭据。
