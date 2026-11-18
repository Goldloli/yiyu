# macOS QA

日期：2026-11-17
版本：0.1.0
机器 / 芯片：MacBook Pro / Apple Silicon
macOS：26 开发环境

## 自动化

- [x] `actionlint`
- [x] `pnpm typecheck`
- [x] `pnpm test`
- [x] `pnpm qa:interactions`
- [x] `pnpm qa:screenshots`
- [x] `pnpm qa:screenshots:compare`
- [x] `cargo test --workspace`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `openspec validate --all --strict`

## 真机

- [x] 首次启动显示 onboarding
- [ ] 完成 onboarding 后再次启动只驻留菜单栏
- [ ] 未授权时提示 Accessibility，并提供系统设置入口
- [ ] Safari 划词成功
- [ ] Chrome 划词成功
- [ ] VS Code 划词成功
- [ ] PDF / Preview 划词成功或显示可操作回退提示
- [ ] 剪贴板回退后原文本 / 图片被恢复
- [ ] 用户在回退期间复制新内容时，新内容不会被旧快照覆盖
- [ ] 浮窗不抢原应用焦点
- [ ] 点击浮窗后可滚动、复制和编辑
- [ ] Esc / 点击外部关闭，固定后保持
- [ ] 全屏 Space 与多显示器定位正确
- [ ] 浅色 / 深色 / Liquid Glass 正常
- [ ] 系统代理、直连、localhost 和手动代理测试
- [x] 关闭主窗口后进程继续驻留
- [x] DMG 生成、挂载、ad-hoc 签名结构验证
- [ ] 从 DMG 安装到 Applications 并完成 Gatekeeper 流程
- [ ] 更新检查、下载、SHA256、打开 DMG

## 结论

通过项：自动化质量门、首次 onboarding、完成配置进入主窗口、关闭主窗口后进程驻留、DMG 结构校验
未通过项：划词快捷键自动化无法由 Computer Use 注入；其余真实应用取词和安装流程待人工验收
阻塞：Windows x64 真机不可用于当前会话
证据路径：`target/release/bundle/dmg/Yiyu_0.1.0_aarch64.dmg`；浏览器截图位于 `web/qa/artifacts/`
