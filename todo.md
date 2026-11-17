# egui 分支 TODO / 进度

- 更新日期：2026-11-17
- 同步基线：`main@faa66a4`（v0.6.5）
- 发布版本：`v0.7.0-alpha.1`（2026-10-21）

## 已完成

### 客户端重写

- 纯 Rust + egui/eframe，移除 Tauri、WebView 与 Vue 前端。
- 工作区、日志、追溯、贮藏、检出、设置六个页面可用。
- egui 专用 CI（fmt + test）与预发布流水线。

### v0.6.5 后端同步（本次）

- 提交时可自动删除已选中的缺失文件（`commit_with_options`，设置项默认关闭）。
- `svn update` 识别文本 / 属性 / 树冲突并给出警告（`parse_update_conflicts`）。
- Windows 目录通知与普通权限增量回退（`status_watch`）。
- 修订号、未版本目录差异、UTF-8 文本差异的健壮性修复。
- `incoming_log` 待拉取提交计算能力（UI 尚未接入）。
- SVN 命令成功但无输出时仍给出结果反馈。
- 状态类型补充 `changelist` 字段（UI 尚未展示）。

### 验证

- `cargo fmt --check` 通过。
- `cargo test`：79 通过，0 失败，2 个性能基准 ignored。

## 待办

### P1 功能对齐（从 `main` 移植 UI）

- [ ] 多项目工作区面板与收藏管理（`main` 5588629）。
- [ ] 更新页 / 待拉取提交列表，接入 `incoming_log`。
- [ ] 复核工作区加载失败的具体错误提示（`main` a3deca7、964e07b）。
- [ ] 日志日期范围筛选；changelist 分组展示。
- [ ] 设置项补充提交模板、最近提交信息等 `main` 已有能力。

### P2 发布

- [ ] 版本号升级到 `0.7.0-alpha.2`，补充发布说明。
- [ ] 走预发布流程，验证 Windows / macOS / Linux 构建产物。

### P3 长期

- [ ] 抽出共享 SVN 后端 crate，避免每次从 `main` 手工复制 `src-tauri/src/svn`。
- [ ] 页面级 / 状态机测试，覆盖提交选择、冲突提示与设置持久化。
- [ ] 界面语言扩展到简中 / 英以外的语言。

## 同步约定

- 后端以 `main` 的 `src-tauri/src/svn` 为上游：复制到 `orcasvn-egui/src/svn`，并同步 `types.rs` 中的类型字段。
- 不直接 merge `main`：该分支已删除 Tauri/Vue 源码，直接合并会带回 4 万余行旧代码。
- 每次同步后更新本文件，并运行 `cargo fmt --check` 与 `cargo test`。
