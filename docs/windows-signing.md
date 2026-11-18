# Windows Signing

第一版 Windows NSIS 安装包未做 Authenticode 签名。

后续正式发行需要：

```text
Windows code signing certificate
signtool sign
SHA-256 timestamp
CI certificate store / secret-backed signing
```

在正式签名完成前，README 和 Release Notes 必须说明 SmartScreen 未知发布者提示。
