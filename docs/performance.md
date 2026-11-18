# Performance Baseline

- 日期：2026-11-17
- 版本：0.1.0
- 机器：MacBook Pro，Apple Silicon，macOS 26 开发环境
- 构建：前端生产构建；Rust debug 常驻采样

## 首屏预算

```bash
pnpm build:web
node scripts/measure-first-load.mjs web/dist
```

当前结果：

- 首屏 JS/CSS raw：303,528 bytes
- 首屏 JS/CSS gzip：93,793 bytes
- CI 预算：175,000 bytes gzip

入口只同步加载 App shell、国际化、主题和基础错误界面。主窗口、设置、onboarding、浮窗与组件画廊均通过动态 import 按窗口加载。

## 空闲驻留

启动应用后采样 5 次，每次间隔 1 秒：

```bash
node scripts/perf-sample.mjs <pid> 5
```

本次 debug 进程结果：

- RSS：137.6 MB，5 次采样保持不变
- CPU：1.3%–1.9%

该数值是开发构建基线，用于发现明显回归，不作为正式发行体积承诺。发布前应对 release 构建重新采样并记录到本文件或对应版本 QA。

## 空闲轮询

生产前端没有常驻 `setInterval` 或定时轮询。设置 revision、翻译增量、权限和更新状态分别由事件或显式动作驱动；历史搜索只在输入后 220ms 防抖一次。更新下载期间才启用进度事件。
