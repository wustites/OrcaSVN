# OrcaSVN (Rust + GPUI Kit，实验分支)

`gpui` 分支上的实验性迁移：把 `egui` 分支的原生客户端改写成 Rust GUI 实现。界面基于 [GPUI Kit](https://github.com/longbridge/gpui-kit)（[GPUI](https://gpui.rs) + 组件层），目的是验证 GPUI 生态能否承载 SVN 客户端这类密集表格 + 文本界面，并评估迁移成本。

- SVN 后端（`src/svn/`：状态缓存、快照、Windows USN 日志、目录通知回退）从 `egui` 分支原样复用。
- 界面层用 GPUI Kit 重写：文本框、按钮基元、主题系统来自 kit；文件列表、差异视图使用 GPUI 的虚拟化列表。
- 当前定位是**原型**：主流程可用，但没有达到 `egui` 分支的完成度，已知差距见下文。

## 运行

```bash
cd orcasvn-gpui
cargo run --release -- /path/to/working-copy   # 路径可选
```

依赖：

- Rust 1.85+（edition 2024；本次验证使用 rustc 1.97）。
- 系统 `svn` 命令行（也可在“设置”中指定可执行文件路径）。
- Linux：需要 `libxkbcommon-x11`、Wayland/X11、fontconfig/freetype 开发库和可用的 Vulkan 驱动；文件对话框走 xdg-desktop-portal（`rfd`）。
- `gpui-kit 0.10` 会带入它固定的 GPUI 快照（crates.io 上的 `gpui-pre 0.3.12`），因此只依赖一个 UI crate，不再直接依赖 `gpui`。

窗口、设置、贮藏与其他平台一致地保存在系统配置目录（`~/.config/orcasvn/`、`~/.cache/orcasvn/`），与 `egui` 分支共用同一份配置。

## 已迁移的功能

| 页面 | 状态 | 说明 |
| --- | --- | --- |
| 工作区 | ✅ | 状态列表（虚拟化、过滤、全选、勾选）、彩色差异、提交（自动 `svn add` 未版本文件、可选自动删除缺失文件）、添加 / 还原 / 删除（带确认弹窗）、更新到 HEAD 或指定版本（冲突提示）、清理、在文件管理器 / VS Code / 终端中打开、每 8 秒自动刷新 |
| 日志 | ✅ | 分页加载、关键字 / 作者搜索（回车提交）、变更路径列表、单文件或整个版本的差异 |
| 追溯 | ✅ | 逐行作者与版本，点击版本高亮同版本行，文件浏览器选择 |
| 贮藏 | ✅ | 补丁粒度贮藏、应用 / 弹出 / 预览 / 删除（带确认弹窗） |
| 检出 | ✅ | URL、本地目录、可选版本 |
| 设置 | ✅ | 简体中文 / English、浅色 / 深色 / 跟随系统、界面缩放、日志条数、提交时自动删除缺失文件、SVN 可执行文件检测 |
| 通用 | ✅ | 命令输出面板、状态栏提示、最近工作副本下拉、启动参数打开工作副本 |

文本输入使用 GPUI Kit 的 `Input` / `Textarea`：支持 IME（中文输入法）、选择、撤销/重做和剪贴板，这是手写输入框无法覆盖的部分。

以下与 `egui` 分支行为不同或缺失：

- 没有右键菜单（差异 / 追溯 / 在文件管理器中显示目前通过按钮完成，解决冲突尚未接入）。
- 面板不可拖拽改宽（左侧列表、日志列表固定宽度）。
- 列表和弹窗仍是自绘的轻量实现，没有用上 kit 的 `DataTable` / `Dock` / `Modal`。
- 没有窗口图标、菜单栏和键盘快捷键。
- `gpui-kit` 固定了 GPUI 快照版本，升级 kit 时需要跟着适配 API。

## 实现要点

```
src/
├── app.rs            应用外壳：顶栏、导航、状态栏、输出面板、页面切换
├── diff.rs           统一差异解析（带行号）+ uniform_list 虚拟化渲染
├── task.rs           tokio 运行时与 GPUI 的桥接
├── ui.rs             主题色板、按钮、复选框、状态徽标、Input/Textarea 封装
├── pages/            工作区 / 日志 / 追溯 / 贮藏 / 检出 / 设置
└── svn/              SVN 后端（复用 egui 分支）
```

- **启动**：`gpui_kit::application()` + `gpui_kit::init` 注册主题、Root、输入法与弹窗基础，再用 `gpui_kit::open_window` 挂载带 Root 的窗口。主题在创建任何输入实体之前根据设置确定（kit 的 `Input` 在构造时固定外观）。
- **异步桥接**：GPUI 使用 smol 执行器，SVN 后端使用 tokio（子进程、超时、`spawn_blocking`）。`task::spawn` 在 GPUI 的后台线程上 `block_on` tokio future，再通过 `WeakEntity` 回到前台线程更新视图。
- **输入框**：`ui::TextField` / `ui::TextArea` 包装 kit 的 `InputState` / `TextareaState`，订阅 `InputEvent` 缓存文本、`PressEnter` 触发搜索；程序化赋值延迟到下一帧（`set_value` 需要 window）。
- **虚拟化**：文件列表、差异和追溯都使用 `uniform_list`，只渲染可见行，适合大工作副本。
- **主题**：kit 提供全局 `Theme`，`ui::palette_from_theme` 把主题色映射成自绘布局使用的 `Palette`，保证自绘区域与 kit 控件一致；界面缩放通过 `window.set_rem_size` 实现。

## 已知问题

- 在没有合成器的 Xvfb 下，首帧要等一次输入事件才显示（真实桌面环境不受影响）。
- 贮藏列表的“应用”与“弹出”行为与 `egui` 版本一致：弹出会应用补丁并删除该贮藏。
- 状态缓存与 `egui` 分支共用，外部修改工作副本后偶尔需要手动点“刷新”。

## 后续工作

1. 列表改用 kit 的 `DataTable`，弹窗改用 kit 的 `Modal`/`Dialog`，分栏改用 `Dock`。
2. 右键菜单、键盘快捷键、窗口图标。
3. 复用 `svn::resolve`，补上“标记冲突已解决”。
4. 为 GPUI 版本添加 CI（`cargo check` / `cargo test` + Linux 依赖）和打包脚本。
5. 与 `egui` 分支共享的分层：把 `svn/`、`settings.rs`、`i18n.rs` 抽成 workspace crate，避免两份拷贝漂移。
