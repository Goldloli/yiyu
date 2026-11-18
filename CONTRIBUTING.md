# Contributing

感谢你愿意改进译羽。提交前请遵守以下流程：

1. 阅读 [AGENTS.md](AGENTS.md)。
2. 新增或修改用户可见行为时，先通过 OpenSpec 创建 change。
3. 等维护者确认规格、设计和任务后，再修改实现代码。
4. 保持 Provider 协议、取词、剪贴板和更新链路的最小权限边界。
5. 不提交 API Key、日志、数据库、构建产物或平台凭据。

提交前至少运行：

```bash
actionlint
pnpm typecheck
pnpm test
pnpm build:web
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
openspec validate --all --strict
```

涉及平台窗口、权限、剪贴板或安装器时，请补充 `docs/qa/` 真机验证记录。CI 通过不能替代对应系统的现场验收。
