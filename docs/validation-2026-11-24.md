# Issue #11 本地复核（2026-11-24）

仓库中已有尚未提交的 Windows Unicode 修复。本次保留这些改动并重新验证，未将线上 Issue 的开放状态等同于本地实现缺失。

## 修复方式

- Windows 默认使用嵌入应用的私有 UTF-8 SVN 运行时。
- 自定义旧 SVN 无法无损接收参数时，在执行工作区命令之前切换到内置运行时，不在变更操作失败后重试。
- 不修改用户已安装的 SVN 或系统区域设置。实现与构建来源见 [Windows SVN 运行时](windows-svn-runtime.md)。

## 本次结果

- `npm run check` 成功：前端类型检查及生产构建通过，104 项前端测试通过，Rust 格式检查通过，Rust 85 项通过、0 项失败、2 项性能测试忽略。
- Windows 真实仓库 Unicode 回归通过：内置默认入口与 PATH 旧客户端入口，覆盖中文、日文、emoji、全角文件名、Unicode 提交消息及 checkout/add/commit/info/diff/revert/log。
- 原 Windows 跳过的中文部分提交、文本 MIME 和嵌套未跟踪差异回归通过。
- 运行时复用、Unicode 缓存目录、缓存损坏恢复与并发准备测试通过。
- `git diff --check` 通过。
- `npm run tauri -- build --no-bundle` 成功；优化构建及内嵌 UTF-8 SVN 的 Windows x64 免安装包已更新到 `release/OrcaSVN-0.6.5-windows-x64-portable.zip`。未执行 NSIS/MSI 安装或桌面交互验收。

本机结果不代表英语 Windows 远程 CI 或 ARM64/x86 原生执行已通过。线上 Issue 应在代码合入、相应 CI 验证及发布后再关闭。
