# Third-Party Notices

译羽的原创代码采用 MIT License。下列直接依赖及其传递依赖保留各自许可证：

| Component | License |
| --- | --- |
| Tauri 2 | Apache-2.0 OR MIT |
| React / React DOM | MIT |
| Vite | MIT |
| Tailwind CSS | MIT |
| Zustand | MIT |
| Lucide | ISC |
| React Markdown / remark-gfm | MIT |
| reqwest | Apache-2.0 OR MIT |
| rusqlite | MIT |
| keyring | Apache-2.0 OR MIT |
| axuielement | Apache-2.0 OR MIT |
| uiautomation | Apache-2.0 |
| clipboard-rs | MIT |
| enigo | MIT |
| atomic-write-file | BSD-3-Clause |
| tracing / tracing-subscriber | MIT |
| semver | Apache-2.0 OR MIT |
| SHA-256 via sha2 | Apache-2.0 OR MIT |

锁文件中包含了完整依赖集合。发布二进制或源码归档前，应从当前 `Cargo.lock` 与 `pnpm-lock.yaml` 生成最终第三方清单。若未来加入 AGPL/GPL 代码、模型或打包运行时，必须单独评估再分发条件。
