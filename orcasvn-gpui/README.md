# OrcaSVN (Rust + GPUI，实验分支)

`gpui` 分支上的实验性迁移：把 `egui` 分支的原生客户端改写成 [GPUI](https://gpui.rs)（Zed 的 GPU 加速 UI 框架）实现。目的是验证 GPUI 能否承载 SVN 客户端这类密集表格 + 文本界面，并评估迁移成本。

- SVN 后端（`src/svn/`：状态缓存、快照、Windows USN 日志、目录通知回退）从 `egui` 分支原样复用。
- 界面层是全新实现：GPUI 没有内置输入框、表格、弹窗，本项目自带最小控件（`src/ui.rs`）和虚拟化列表。
- 当前定位是**原型**：主流程可用，但没有达到 `egui` 分支的完成度，已知差距见下文。

## 运行

```bash
cd orcasvn-gpui
cargo run --release -- /path/to/working-copy   # 路径可选
```

依赖：

- Rust 1.85+（`gpui 0.2.2` 使用 edition 2024；本次验证使用 rustc 1.97）。
- 系统 `svn` 命令行（也可在“设置”中指定可执行文件路径）。
- Linux：需要 `libxkbcommon-x11`、Wayland/X11、fontconfig/freetype 开发库和可用的 Vulkan 驱动；文件对话框走 xdg-desktop-portal（`rfd`）。
- `gpui` 依赖的 `gpui_http_client -> zed-async-tar -> xattr 0.2` 仍在引用 `libc::ENOATTR`，而 `libc 0.2.190+` 已移除该常量，因此 `Cargo.toml` 暂时固定 `libc = "=0.2.189"`。

窗口、设置、贮藏与其他平台一致地保存在系统配置目录（`~/.config/orcasvn/`、`~/.cache/orcasvn/`），与 `egui` 分支共用同一份配置。

## 已迁移的功能

| 页面 | 状态 | 说明 |
| --- | --- | --- |
| 工作区 | ✅ | 状态列表（虚拟化、过滤、全选、勾选）、彩色差异、提交（自动 `svn add` 未版本文件、可选自动删除缺失文件）、添加 / 还原 / 删除（带确认弹窗）、更新到 HEAD 或指定版本（冲突提示）、清理、在文件管理器 / VS Code / 终端中打开、每 8 秒自动刷新 |
| 日志 | ✅ | 分页加载、关键字 / 作者搜索、变更路径列表、单文件或整个版本的差异 |
| 追溯 | ✅ | 逐行作者与版本，点击版本高亮同版本行，文件浏览器选择 |
| 贮藏 | ✅ | 补丁粒度贮藏、应用 / 弹出 / 预览 / 删除（带确认弹窗） |
| 检出 | ✅ | URL、本地目录、可选版本 |
| 设置 | ✅ | 简体中文 / English、浅色 / 深色 / 跟随系统、界面缩放、日志条数、提交时自动删除缺失文件、SVN 可执行文件检测 |
| 通用 | ✅ | 命令输出面板、状态栏提示、最近工作副本下拉、启动参数打开工作副本 |

以下与 `egui` 分支行为不同或缺失：

- **没有 IME / 输入法支持**：文本框直接读键盘字符，中文（以及任何需要输入法的文字）无法通过键盘输入，提交信息等只能输入 ASCII。GPUI 需要自行实现 `EntityInputHandler`（官方 `examples/input.rs` 约 700 行）才能支持 IME，属于后续工作。
- 没有右键菜单（差异 / 追溯 / 在文件管理器中显示 / 解决冲突目前通过按钮完成，解决冲突尚未接入）。
- 面板不可拖拽改宽（左侧列表、日志列表固定宽度）。
- GPUI 没有全局主题层：颜色来自 `ui::palette`，`跟随系统` 直接读窗口外观；系统切换深浅色后会随下一次重绘生效。
- 没有窗口图标、菜单栏和键盘快捷键。
- `gpui 0.2.2` 是 crates.io 上的发布版本，API 与 Zed 主分支的 GPUI 有差异，升级时需要重新适配。

## 实现要点

```
src/
├── app.rs            应用外壳：顶栏、导航、状态栏、输出面板、页面切换
├── diff.rs           统一差异解析（带行号）+ uniform_list 虚拟化渲染
├── task.rs           tokio 运行时与 GPUI 的桥接
├── ui.rs             调色板、按钮、复选框、状态徽标、最小文本框
├── pages/            工作区 / 日志 / 追溯 / 贮藏 / 检出 / 设置
└── svn/              SVN 后端（复用 egui 分支）
```

- **异步桥接**：GPUI 使用 smol 执行器，SVN 后端使用 tokio（子进程、超时、`spawn_blocking`）。`task::spawn` 在 GPUI 的后台线程上 `block_on` tokio future，再通过 `WeakEntity` 回到前台线程更新视图。
- **输入框**：`ui::TextField` 保存字符串 + 字符游标，渲染时插入 `│` 光标；编辑键（退格、方向键、Home/End）和普通字符在 `on_key_down` 中处理。所有文本框都是焦点元素，点击获得焦点。
- **虚拟化**：文件列表、差异和追溯都使用 `uniform_list`，只渲染可见行，适合大工作副本。
- **主题**：GPUI 没有内置主题层，`ui::palette(dark)` 提供一套浅色/深色色板，页面通过 `Palette` 取色；界面缩放通过 `window.set_rem_size` 实现。

## 已知问题

- 在没有合成器的 Xvfb 下，首帧要等一次输入事件才显示（真实桌面环境不受影响）。
- 贮藏列表的“应用”与“弹出”行为与 `egui` 版本一致：弹出会应用补丁并删除该贮藏。
- 状态缓存与 `egui` 分支共用，外部修改工作副本后偶尔需要手动点“刷新”。

## 后续工作

1. 接入 `EntityInputHandler`，支持 IME 与粘贴。
2. 可拖拽分栏、右键菜单、键盘快捷键。
3. 复用 `svn::resolve`，补上“标记冲突已解决”。
4. 为 GPUI 版本添加 CI（`cargo check` / `cargo test` + Linux 依赖）和打包脚本。
5. 与 `egui` 分支共享的分层：把 `svn/`、`settings.rs`、`i18n.rs` 抽成 workspace crate，避免两份拷贝漂移。
