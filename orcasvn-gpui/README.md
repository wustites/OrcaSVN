# OrcaSVN (Rust + GPUI Kit，实验分支)

`gpui` 分支上的实验性迁移：把 **`main` 分支的 Tauri + Vue 客户端**迁移到原生 Rust + GPUI。功能与信息架构以 `main` 为准，`egui` 分支只作为早期脚手架，不再是参考对象。

- `orcasvn-core/`：从 `main` 的 `src-tauri/src/svn` 抽出的共享 SVN 后端（状态缓存、快照、Windows USN、目录通知回退）。Tauri 客户端和 GPUI 客户端现在共用这一份实现。
- `orcasvn-gpui/`：GPUI 客户端，界面基于 [GPUI Kit](https://github.com/longbridge/gpui-kit)（GPUI + 组件层）。
- `src-tauri/`：原有 Tauri 客户端，已改为依赖 `orcasvn-core`。

当前定位是**实验版本**：主流程可用，尚未完全追平 `main` 的细节（见“已知差距”）。

## 运行

```bash
cd orcasvn-gpui
cargo run --release -- /path/to/working-copy   # 路径可选
```

依赖：

- Rust 1.88+（edition 2024，`slice::as_chunks`；本次验证 rustc 1.97 / CI stable 1.100）。
- 系统 `svn` 命令行（也可在“设置”中指定可执行文件路径）。
- Linux：需要 `libxkbcommon-x11`、Wayland/X11、fontconfig/freetype 开发库和可用的 Vulkan 驱动；文件对话框走 xdg-desktop-portal（`rfd`）。
- `gpui-kit 0.10` 带入固定的 GPUI 快照（crates.io 上的 `gpui-pre 0.3.12`），应用只依赖一个 UI crate。

配置保存在 `~/.config/orcasvn/`（与 Tauri 版本共用配置目录）。

## 已迁移的功能（对齐 main）

| 页面 | 状态 | 说明 |
| --- | --- | --- |
| 多项目工作区栏 | ✅ | 最近工作副本、变更数、最近修改时间、收藏、备注、折叠、手动排序/按名称/按变更、上移下移、切换/打开 |
| 工作区 | ✅ | `DataTable` 状态列表（虚拟滚动、列宽拖拽、按路径排序）、过滤/全选、彩色差异、提交（自动 `svn add`、可选自动删除缺失文件）、添加 / 还原 / 删除（kit 对话框）、右键菜单（差异 / 追溯 / 显示 / 标记冲突已解决）、自动刷新 |
| 提交 | ✅ | 独立页面：目录 scope（右键“提交此文件/此目录”进入）、勾选文件、提交信息、差异预览、提交 |
| 更新 | ✅ | 本地/远端版本、目标版本、检查更新（incoming log）、更新并提示冲突 |
| 差异 | ✅ | 独立页面：工作副本 / 指定版本 / 版本区间三种对比模式 |
| 日志 | ✅ | 关键字 / 作者 / **日期范围**筛选、分页、单文件或整版差异 |
| 追溯 | ✅ | 逐行作者与版本，点击版本高亮 |
| 贮藏 | ✅ | 补丁粒度贮藏、应用 / 弹出 / 预览 / 删除（kit 对话框） |
| 检出 | ✅ | URL、本地目录、可选版本 |
| 设置 | ✅ | 中英文、深浅色、界面缩放、日志条数、自动删除缺失文件、SVN 可执行文件检测、收藏与备注管理、项目栏折叠 |
| 交互 | ✅ | 可拖拽分栏、命令输出面板、路由标题栏、底部路径/变更数/版本栏、全局快捷键 |

文本输入使用 GPUI Kit 的 `Input` / `Textarea`，支持 IME、选择、撤销/重做和剪贴板。

快捷键（GPUI actions 全局注册，输入框内也可用）：

| 快捷键 | 动作 |
| --- | --- |
| `Ctrl+O` | 打开工作副本 |
| `Ctrl+R` / `F5` | 刷新状态 |
| `Ctrl+F` | 聚焦文件过滤框 |
| `Ctrl+Enter` | 提交已选文件 |
| `Ctrl+1` … `Ctrl+9` | 工作区 / 日志 / 提交 / 更新 / 差异 / 贮藏 / 追溯 / 检出 / 设置 |

## 与 main 的已知差距

- 多项目栏用 ↑/↓ 与排序按钮代替 `main` 的拖拽排序。
- 分栏使用 `Resizable` 而不是 `Dock`：没有标签页、拖拽停靠和布局持久化。
- 提交页的目录 scope 通过右键进入，尚未提供 `main` 的路由式目录树选择。
- 视觉使用 GPUI Kit 主题，不复刻 Element Plus 的具体样式。

## 实现要点

```
orcasvn-core/          共享 SVN 后端（自 main/src-tauri 抽取）
orcasvn-gpui/src/
├── app.rs             外壳：工具栏、项目栏、分组导航、路由标题、页脚
├── diff.rs            差异解析 + 虚拟化渲染
├── task.rs            tokio 与 GPUI 的桥接
├── ui.rs              主题色板、按钮、复选框、Input/Textarea 封装
├── pages/
│   ├── sidebar.rs     多项目工作区栏（收藏/备注/摘要）
│   ├── workspace.rs   工作区
│   ├── commit.rs      提交
│   ├── update.rs      更新
│   ├── diff.rs        差异
│   ├── log.rs         日志（含日期范围）
│   └── blame / stash / checkout / settings
└── system.rs          文件管理器 / 编辑器 / 终端集成
```

- **共享后端**：`orcasvn-core` 提供 `svn/` 模块与数据类型；Tauri 与 GPUI 都依赖它，避免三份拷贝漂移（egui 分支后续也可切换）。
- **异步桥接**：SVN 后端使用 tokio；GPUI 侧在后台线程 `block_on` 后通过 `WeakEntity` 回到前台更新视图。
- **项目栏摘要**：每个最近工作副本异步读取 `cached_status` 与 `cached_latest_modified`，60 秒刷新一次。
- **快捷键**：`actions!` + `cx.bind_keys` + `App::on_action`，页面切换丢失焦点后仍然有效。

## CI 与预发布

- `.github/workflows/gpui-ci.yml`：`gpui` 分支与相关 PR 跑 core/gpui 的格式检查、Clippy（`-D warnings`）、单元测试，检查 Tauri 后端，并在 Windows / macOS 上做 `cargo check`。
- `.github/workflows/gpui-prerelease.yml`：推送 `gpui-vX.Y.Z-alpha.N`（也支持 `-beta.N` / `-rc.N`）校验版本、创建 draft、三平台打包上传并自动发布。
- 发布产物同时包含安装包与压缩包：
  - Linux：`.deb`、`.AppImage`、`.tar.gz`
  - Windows：NSIS 安装程序（`-setup.exe`）、`.zip`
  - macOS：`.dmg`、`.app.zip`、`.tar.gz`
- CI 的 `Linux installers` job 会构建 `.deb` / `.AppImage` 并作为 workflow artifact 上传，PR 也能下载验证。
- 安装包未签名：macOS 首次打开需右键“打开”或在“隐私与安全性”中允许，Windows 可能触发 SmartScreen 提示。

```bash
git tag gpui-v0.8.0-alpha.1
git push origin gpui-v0.8.0-alpha.1
```

## 后续工作

1. 多项目栏拖拽排序（当前为 ↑/↓ + 排序模式）。
2. 分栏升级为 `Dock`（标签页、停靠、布局持久化）。
3. 窗口图标、菜单栏、自定义快捷键；macOS Intel 与 Linux aarch64 产物。

## 相关文档

- [引入 gpui-kit-material 的可行性评估](../docs/gpui-material-migration.md)：依赖特性、MSRV、图标体积、风险与分阶段方案。尚未实施。
