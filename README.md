# OrcaSVN

[English](README.en.md) | 简体中文

OrcaSVN 是一个基于 Tauri、Rust 和 Vue 3 的跨平台 SVN 桌面客户端。它借鉴 Git 客户端清晰的工作区体验，同时保留 SVN 的集中式版本控制语义。

![OrcaSVN 软件界面](docs/images/orcasvn-workspace.png)

## 核心能力

- 以类似 `git status` 的分类查看本地变更、未版本文件、冲突和缺失文件
- Checkout、Update、Commit、Add、Delete、Revert、Cleanup、Switch 和 Merge
- 查看提交历史、文件差异和逐行 Blame
- 支持简体中文、繁体中文、英语、日语和韩语
- 支持浅色、深色主题以及 Windows、macOS、Linux

> OrcaSVN 调用本机的 `svn` 命令行工具，不会自行实现 SVN 协议。

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
2. 在工作区按“变更、未版本、冲突、缺失”筛选文件。
3. 选择文件查看 Diff，确认后进入 Commit 页面提交。
4. 提交前先执行 Update，并优先解决冲突。

更完整的操作说明见 [QUICKSTART.md](QUICKSTART.md)。

## 本地开发

要求：

- Node.js 18 或更高版本
- 最新稳定版 Rust
- SVN CLI 1.10 或更高版本
- 平台对应的 Tauri 2 构建依赖

```bash
npm ci
npm run tauri dev
```

提交改动前运行：

```bash
npm run check
```

详细环境配置和常见问题见 [SETUP.md](SETUP.md)，贡献约定见 [CONTRIBUTING.md](CONTRIBUTING.md)。

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

## 许可证

[MIT](LICENSE)
