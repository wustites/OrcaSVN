# OrcaSVN (Rust + egui)

`egui` 分支上的 OrcaSVN 重写版本：用纯 Rust + [egui](https://github.com/emilk/egui)/eframe 实现界面，去掉 Tauri、WebView 与 Vue 前端，单个原生可执行文件即可运行。

SVN 后端（`src/svn/`，包括状态缓存、快照与 Windows USN 日志）直接沿用自 `src-tauri/src/svn`。

## 运行

```bash
cd orcasvn-egui
cargo run --release -- /path/to/working-copy   # 路径可选
```

需要系统中可用的 `svn` 命令行（也可在“设置”中指定可执行文件路径）。
Linux 下 `rfd` 文件对话框依赖 GTK3（`libgtk-3-dev`）。

## 功能

| 页面 | 功能 |
| --- | --- |
| 工作区 | 状态列表（自动刷新、过滤、勾选）、彩色差异、提交（自动 `svn add` 未版本文件）、添加 / 还原 / 删除、解决冲突、更新到 HEAD 或指定版本、清理、在文件管理器 / VS Code / 终端中打开 |
| 日志 | 分页加载、关键字 / 作者搜索、变更路径列表、单文件或整个版本的差异 |
| 追溯 | 逐行作者与版本，点击版本高亮同版本行 |
| 贮藏 | 将所选文件的修改存为补丁并还原；应用 / 弹出 / 预览 / 删除 |
| 检出 | URL、本地目录、可选版本 |
| 设置 | 简体中文 / English、浅色 / 深色 / 跟随系统、界面缩放、日志条数、SVN 可执行文件 |

设置与贮藏保存在系统配置目录（如 `~/.config/OrcaSVN/`），状态缓存位于系统缓存目录。

## 与 Tauri 版本的差异

- 界面语言目前只有简体中文与英文。
- 贮藏以文件为粒度（不支持按 hunk 选择），且不包含未版本文件。
- 中文字体从系统加载（微软雅黑 / 苹方 / Noto Sans CJK / 文泉驿）。
