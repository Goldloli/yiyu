# Manual QA

CI 负责编译、单元测试、浏览器 mock 交互与截图。以下能力必须在对应真机验证：

截图基准来自 GitHub Actions 的 macOS 15 运行器。其他 macOS 版本的系统字体渲染可能略有不同，需要结合差异图人工判断。

- macOS Accessibility 权限、任意应用取词、浮窗焦点和全屏 Space
- Windows UI Automation 取词、剪贴板恢复和 non-activating popup
- 系统代理、PAC、手动 HTTP / HTTPS / SOCKS5
- NSIS / DMG 安装、Gatekeeper / SmartScreen
- 更新下载、取消、SHA256 校验和安装器启动

记录模板见 `macos.md` 与 `windows.md`。未实际执行的项目保持未勾选。
