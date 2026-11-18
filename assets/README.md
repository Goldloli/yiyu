# 应用图标

`icon.svg` 是桌面应用图标的生成入口，引用同目录的 `icon-source.png`；SVG 负责图标轮廓和透明边距，PNG 保存选定的双羽图稿。

可使用 `pnpm tauri icon assets/icon.svg` 重新生成 `src-tauri/icons/` 下的平台资源。该命令也会生成移动端图标；当前项目仅发布 macOS 和 Windows。
