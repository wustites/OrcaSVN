# 开发环境

## 前置依赖

安装并验证 Node.js、Rust 和 SVN CLI：

```bash
node --version
npm --version
rustc --version
cargo --version
svn --version --quiet
```

Windows 还需要 Visual Studio C++ Build Tools 和 WebView2。

Linux（Debian/Ubuntu）还需 Tauri 2 的系统包，并安装 Subversion：

```bash
sudo apt-get update
sudo apt-get install -y pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf build-essential \
  libssl-dev subversion
```

macOS 与其他发行版的图形库请参考 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)。

## 启动项目

```bash
npm ci
npm run tauri dev
```

仅调试前端时可以运行 `npm run dev`，但所有 SVN 操作都依赖 Tauri 后端。

## 验证改动

```bash
npm run check
```

该命令依次检查前端类型和构建、运行前端单元测试，再检查 Rust 格式并运行 Rust 单元测试。提交前还可以单独运行：

```bash
npm run build
npm run test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 发布构建

```bash
npm run release
```

Windows 完整构建成功后，最新程序、安装包和免安装 ZIP 会自动汇总到项目根目录 `release/`，同时生成包含构建时间、产物路径和 SHA-256 的 `release-manifest.json`。原来的 `npm run tauri -- build` 同样会自动汇总；仅运行 `npm run build` 只编译前端，不生成桌面发布包。直接调用 `cargo build` 或 `npx tauri build` 不经过本项目的发布汇总脚本。

`release/` 只更新清单记录的自动生成文件，用户另行放入的文件会保留。同名的非托管文件会阻止覆盖并提示处理。构建失败不会替换已有发布文件；先生成完整暂存包，再更新最新目录。旧版本正在运行导致文件占用时，旧目录可能保留为 `.release-previous-*`，退出旧程序后可自行删除。

建议从 `release/OrcaSVN.exe` 运行程序，避免锁住 Cargo 编译目标。若旧程序仍从 `src-tauri/target/release/` 运行，请先退出，或设置 `CARGO_TARGET_DIR` 到另一个构建缓存目录后再构建；相对目录以 `src-tauri/` 为基准。`--no-bundle` 只汇总程序及免安装包；调试构建、自定义 profile 和非 Windows 平台保留 Tauri 默认输出，不更新根目录 `release/`。

推送 `vX.Y.Z` 标签（例如 `v0.6.3`）时，正式发布流水线校验全部版本文件、运行三平台检查并打包，全部成功后公开 Release 并标记为 Latest，详见[发布流程](docs/releasing.md)。预发布标签（例如 `v0.6.3-rc.1`）由独立流水线处理，详见[预发布说明](docs/prerelease.md)。

## 排查问题

### 找不到 SVN

确保 `svn` 位于当前用户的 `PATH` 中，然后重新启动 OrcaSVN。

### Windows 编译失败

确认 Rust 使用 MSVC 工具链，并安装 Visual Studio 的“使用 C++ 的桌面开发”工作负载。

### 端口 1420 被占用

开发服务器要求固定使用 1420 端口。结束占用进程后重新运行 `npm run tauri dev`。
