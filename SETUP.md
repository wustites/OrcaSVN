# 开发环境设置指南

## 必需的安装

### 1. 安装 Rust

访问 [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) 下载并安装 Rust。

Windows 用户可以直接运行以下命令（需要 PowerShell）：

```powershell
winget install Rustlang.Rust.MSVC
```

或者下载安装程序：[rustup-init.exe](https://static.rust-lang.org/rustup/dist/i686-pc-windows-gnu/rustup-init.exe)

安装完成后，验证：

```bash
rustc --version
cargo --version
```

### 2. 安装 Node.js

访问 [https://nodejs.org/](https://nodejs.org/) 下载并安装 Node.js（推荐 LTS 版本）。

验证：

```bash
node --version
npm --version
```

### 3. SVN 命令行工具

你已经安装了 SVN，版本 1.14.5。

## 开始开发

1. **安装依赖**

```bash
npm install
```

2. **启动开发模式**

```bash
npm run tauri dev
```

首次运行时会下载和编译 Rust 依赖，可能需要几分钟。

3. **构建发布版本**

```bash
npm run tauri build
```

构建产物位于 `src-tauri/target/release/` 目录。

## 常见问题

### Rust 编译错误

确保安装了最新的 Rust 工具链：

```bash
rustup update
```

### WebView2 错误

Windows 10 1803+ 已内置 WebView2。如果是旧版本 Windows，请下载安装：
[Microsoft Edge WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### 端口占用

开发服务器默认使用 1420 端口。如果被占用，修改 `tauri.conf.json` 中的 `devPath`。
