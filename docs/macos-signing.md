# macOS Signing and Notarization

第一版开发构建使用 ad-hoc 签名，不包含 Apple Developer ID 公证。

macOS 依据代码签名识别辅助功能授权对象。ad-hoc 签名随构建内容变化，重新构建后旧授权可能无法匹配。

### 无 Apple 开发者账号的本地开发

以下命令为当前用户创建一次自签名开发身份，并使用它构建调试应用。私钥保存在默认钥匙串，证书不加入系统信任；重复执行配置命令会复用已有身份：

```bash
pnpm macos:signing:setup
pnpm build:macos:local
```

普通的 `pnpm tauri build` 仍使用项目默认的 ad-hoc 签名。首次切换到这个签名身份后，需要在系统设置中为当前应用重新授权辅助功能；之后使用同一身份构建，系统可以识别其代码身份。不要删除或重新生成该证书，否则需要重新授权。此方式仅用于本机开发，不具备 Apple 身份认证，也不能用于公证或正式分发。

### Apple 开发签名

如果已拥有 Apple Development 签名身份，也可以直接指定它：

```bash
security find-identity -v -p codesigning
APPLE_SIGNING_IDENTITY="钥匙串中的签名身份" pnpm tauri build --debug --bundles app
```

`APPLE_SIGNING_IDENTITY` 覆盖项目默认的 ad-hoc 配置。免费开发者账户可用于本地开发签名，但不能用于公证；正式分发需要 Developer ID。参见 [Tauri macOS 签名文档](https://v2.tauri.app/distribute/sign/macos/)和 [Apple 的代码签名要求说明](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements)。

后续正式发行需要：

```text
Apple Developer Program
Developer ID Application certificate
notarytool credentials
hardened runtime
DMG signing
staple notarization ticket
```

在正式签名完成前，README 和 Release Notes 必须持续说明 Gatekeeper 放行方式，不能声称应用已公证。
