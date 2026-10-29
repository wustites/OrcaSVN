# OrcaSVN

简体中文 | [繁體中文](README.zh-TW.md) | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md)

OrcaSVN 是一个基于 Tauri、Rust 和 Vue 3 的跨平台 SVN 桌面客户端。它借鉴 Git 客户端清晰的工作区体验，同时保留 SVN 的集中式版本控制语义。

![OrcaSVN 软件界面](docs/images/orcasvn-workspace.png)

0.6.3 发布前开发界面，2026-10-02 截取；使用示例工作区数据。[深色主题](docs/images/orcasvn-workspace-dark.png)。

## 核心能力

- 以类似 `git status` 的分类查看本地变更、未版本文件、冲突和缺失文件
- 检出、更新、按文件或目录选择提交、加入版本控制、还原和清理工作副本
- 本地贮藏：按文件或文本分块保存修改，并恢复到原工作区
- 查看提交历史、文件差异和逐行 Blame
- 支持简体中文、繁体中文、英语、日语和韩语
- 支持浅色、深色主题以及 Windows、macOS、Linux

> OrcaSVN 调用本机的 `svn` 命令行工具，不会自行实现 SVN 协议。

Switch、Merge、Resolve 和受控文件安排删除目前需使用其他 SVN 客户端或命令行完成，再回到 OrcaSVN 刷新。工作区中的“删除”按钮用于删除未跟踪的本地文件。

## 安装

### Windows

```powershell
winget install OrcaSVN.OrcaSVN
```

也可以从 [GitHub Releases](https://github.com/wustites/OrcaSVN/releases) 下载 Windows、macOS 或 Linux 安装包。

安装后请确认 SVN CLI 可用：

```bash
svn --version --quiet
```

## 快速开始

1. 打开 OrcaSVN，选择已有 SVN 工作副本，或通过 Checkout 检出仓库。
2. 在工作区按“变更、未跟踪、冲突、缺失”筛选文件。
3. 选择文件查看 Diff，确认后进入 Commit 页面提交。
4. 提交前先执行 Update，并优先解决冲突。

### 按目录部分提交

工作区以平铺文件列表展示变更。进入提交页后，勾选目录即可联动勾选其下所有可提交子项，也可以逐项取消；顶部显示提交范围和已选总数，搜索隐藏的已选项仍计入提交。查看 Diff 后返回会保留勾选和提交信息。

未跟踪目录先通过“加入版本控制并预览”展开文件列表，检查后再提交。删除或替换目录必须完整选择其子项；复制或移动目录暂不支持部分提交，会提示使用 SVN 客户端处理完整操作。

入门步骤见 [快速使用](QUICKSTART.md)，完整操作与常见问题见 [用户手册](docs/user-guide.md)。

日志页默认每页 20 条，可按作者、关键词和日期范围筛选；长作者名和提交信息省略显示，悬停查看全文。打开已缓存工作区时会先显示旧列表，再验证最新状态，未验证时显示提示并限制相关变更操作。

## 本地开发

要求：

- Node.js 18 或更高版本
- Rust **1.88** 或更高版本。依赖使用 Rust edition 2024，1.85 起才能编译这类包；本仓库还会用到 1.88 稳定的 `slice::as_chunks`。1.83 及更早版本无法通过 `cargo check`。
- SVN CLI 1.10 或更高版本（含 `svnversion`，用于读取工作副本中已有的最高版本）
- 平台对应的 Tauri 2 构建依赖

Linux（Debian/Ubuntu）系统包示例：

```bash
sudo apt-get install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf build-essential \
  libssl-dev subversion
```

```bash
npm ci
npm run tauri dev
```

提交改动前运行：

```bash
npm run build
npm run test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

`npm run check` 会依次做前端类型检查与构建、前端单元测试、Rust 格式检查和 `cargo test`。

详细环境配置和常见问题见 [SETUP.md](SETUP.md)，贡献约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。

### 工作副本行为

- macOS 与 Linux 上，SVN 子进程使用 UTF-8 locale（macOS 为 `en_US.UTF-8`，Linux 为 `C.UTF-8`），避免桌面启动时继承缺失、ASCII 或无效 locale 导致中文路径、仓库 URL、提交说明和命令输出乱码。这对应 [#7](https://github.com/wustites/OrcaSVN/issues/7)。
- 界面上的本地版本取工作副本里已经存在的最高版本（`svnversion` 的上界）。按文件部分提交仍使用 `--depth empty`，但刚提交的版本不会再被更新页当成尚未应用到本地。
- 加入版本控制时，如果 SVN 把有效 UTF-8 文本自动标成 `application/octet-stream`，会去掉该属性，使差异和 Blame 仍能显示正文。含 NUL 或非法 UTF-8 的二进制文件保持原样。
- 检出结果按成功或失败状态显示，不靠错误文本里是否出现英文字符串 `Error`。
- 更新若产生文本、属性或树冲突，会列出冲突路径并给出警告。SVN 在这种情况下仍以退出码 0 结束。
- 未跟踪目录中的文件也可以查看差异，内容按新增显示。文件一旦不再出现在状态列表中，工作区会清掉对应的差异预览。

当前正式版本为 **0.6.4**，更新内容见 [发布说明](docs/releases/v0.6.4.md)。正式发布步骤见 [发布流程](docs/releasing.md)，候选版本见 [预发布说明](docs/prerelease.md)，与 v0.6.2 的性能对照及测量边界见 [性能基准](docs/workspace-startup-benchmark.md)。

## 项目结构

```text
src/                    Vue 3 前端
  api/                  Tauri 命令调用封装
  composables/          可复用工作区逻辑
  i18n/                 多语言资源
  stores/               Pinia 状态
  views/                页面
src-tauri/src/
  main.rs               Tauri 命令边界
  svn/executor.rs       SVN 进程执行与超时
  svn/operations.rs     SVN 参数构造
  svn/parser.rs         XML/文本结果解析
.github/workflows/      发布流水线
```

## 设计原则

- **可预测**：界面操作尽量对应明确的 SVN 命令。
- **先审阅后变更**：默认先展示状态和差异，再执行提交或还原。
- **安全参数边界**：文件目标使用 `--` 与命令选项隔离。
- **清晰反馈**：错误保留 SVN 原始上下文，便于诊断。

界面颜色、文字、间距与组件尺寸见 [视觉规范](docs/visual-design-system.md)。

## 许可证

[MIT](LICENSE)
