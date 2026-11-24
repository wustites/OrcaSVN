# Windows UTF-8 SVN 运行时

Windows 版默认使用嵌入 OrcaSVN 可执行文件的私有 SVN。它解决旧版 Windows SVN 在进入命令处理前将参数转换到 ANSI 代码页，导致中文、emoji 或近似字符丢失的问题。

## 来源与构建

基于 SlikSVN 1.14.5，保留其 DLL、许可证与 Notice。仅修改应用私有副本的 `svn.exe`、`svnadmin.exe` 进程清单，声明 `activeCodePage=UTF-8`，不改可执行代码或用户已安装的 SVN。修改后的文件不是原厂签名的原始二进制，随程序嵌入并由 OrcaSVN 内的运行时数据进行完整性校验。

- 上游下载：[SlikSVN](https://sliksvn.com/download/)。
- 上游源码：[Apache Subversion 1.14.5](https://github.com/apache/subversion/tree/1.14.5)。
- [SlikSVN 的再分发说明](https://sliksvn.com/support-articles/can-i-embed-the-sliksvn-client-in-my-product/)。
- [微软的进程 UTF-8 代码页文档](https://learn.microsoft.com/en-us/windows/apps/design/globalizing/use-utf8-code-page)。要求 Windows 10 1903 或更新版本。

下载包的固定 SHA-256：

| 架构 | 上游归档 SHA-256 |
| --- | --- |
| x64 | `77D4FE02999DDA3BDC3A20E86243AE6EDE99AAF072B4C12B0CDEDB54D88E954A` |
| ARM64 | `A912BFF8358BBDE801E1CF4B0B32B80A977BC0B3A5E06E1BC726DE05D0E600B6` |
| x86 | `033A4CFD484B6BB43F08D52D99C93748C40E2CB3BCBC30B63912872B6EB8CACC` |

执行 `npm run prepare:svn` 会校验下载归档、通过 MSI 管理提取到暂存目录、保留原清单设置并加入 UTF-8 声明，然后生成 `src-tauri/runtime/windows-<arch>.zip`。需要 Windows SDK 中的 `mt.exe`。管理提取不安装产品、不更新 PATH。生成物已排除出 Git；缓存收据同时核对生成脚本、上游包和生成归档的 SHA-256。

`npm run tauri -- dev`、`npm run tauri -- build`、`npm run release` 与 `npm run check` 会自动准备运行时。直接运行 Cargo 前先准备；交叉编译时可执行 `node scripts/prepare-svn.cjs --target aarch64-pc-windows-msvc`（x86 为 `i686-pc-windows-msvc`）。编译时根据目标架构嵌入对应归档；安装包和免安装版均不依赖额外的 SVN 文件或首次运行联网下载。

## 执行与缓存

第一次执行 SVN 时，解包到 `%LOCALAPPDATA%/OrcaSVN/svn-runtime/<归档SHA-256>/`。读取缓存时逐文件比对内嵌数据，拒绝额外 DLL，并验证暂存目录后原子发布。多个进程可并发准备同一版本；旧缓存损坏或被占用时使用新的校验副本，不覆盖旧目录。

默认 SVN 始终来自该运行时。自定义 SVN 参数可无损表示时继续使用用户选择；超出系统代码页时先进行独立本地能力检测，确认不支持后在工作区操作前切换到内置运行时。不会失败后重试已执行的变更命令。用户的 SVN 配置和认证缓存仍按 SVN 默认规则使用。

## 验证范围

本机 Windows x64、ANSI 936：真实仓库测试要求中文与 emoji 工作区、中文/日文/emoji 文件名、全角与半角近似文件名、Unicode 提交消息的 checkout/add/commit/info/diff/revert/log 全部成功。既验证默认内置入口，也验证 PATH 中的旧 SVN 被明确选择时的兼容执行。原先被 Windows 跳过的中文部分提交、MIME、嵌套未跟踪差异测试已恢复。

另测运行时复用、中文/emoji 缓存路径、损坏缓存恢复和并发发布。x64、ARM64、x86 归档准备均已验证；ARM64/x86 原生执行及英语 Windows 由相应环境进一步验收，不能用 x64 本机结果替代。Windows PR CI 安装旧 SlikSVN 并运行完整 Rust 回归。
