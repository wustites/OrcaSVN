# 2026-11-16 修复与验收记录

环境：Windows、本机 TortoiseSVN 1.14.5、ANSI 代码页 936。

## 已实现

- 工作区清空时递增会话；加载、刷新、请求复用和刷新时间缓存检查会话，避免移除后立即重加复用旧任务。
- 发布汇总按 RFC 7396 合并基础配置、Windows 平台配置和命令行 JSON 覆盖。JSON5/TOML、外部 TAURI_CONFIG、产品/二进制名称变更目前明确报错，不回退到旧版本。
- 根据最终配置和 `--bundles` 确定必须生成的安装包。版本不匹配、目录缺失、产物过期均在替换旧发布前报错；明确禁用 bundle 时允许只发布免安装包。
- 所有访问共享 WORKSPACES 的缓存模块测试使用同一互斥锁，防止三工作区 LRU 容量造成跨测试淘汰。测试运行器仍采用默认并行设置；该模块内共享缓存测试串行执行，未改动生产缓存容量。
- Linux CI 安装 subversion 并校验 svn / svnadmin，避免缺工具导致真实 fixture 提前返回。

## 验证

- `npm run build`：通过。
- `npm run test:unit`：104 项通过；新增可控延迟重开、配置覆盖及发布完整性测试。
- Windows 原生 `Compress-Archive`：覆盖无 bundle、NSIS、MSI、NSIS+MSI，检查清单大小及 SHA-256、保留用户文件、阻止覆盖非托管同名文件。
- 发布目录重命名故障注入：验证恢复旧清单和用户文件并清理暂存目录。故障注入不能代替真实 Windows 文件占用验收。
- 最终 `npm run check`：构建、104 项单测、格式检查、Rust 85 通过 / 0 失败 / 2 忽略。缓存测试隔离改动后曾连续运行默认 Rust 测试三次，每次 80 通过；新增 Unicode 和运行时回归后的最终计数为 85。未宣称被忽略的性能测试通过。
- `npm run tauri -- build --no-bundle`：Windows 原生优化构建及含内嵌运行时的免安装 ZIP 生成通过。未执行本次 NSIS/MSI 安装、卸载验收。

## Issue #11

本地功能修复完成：Windows 使用内嵌的私有 UTF-8 SVN 运行时，旧自定义客户端在参数不可表示时预先切换至内置运行时。完整实现与来源见 [Windows SVN 运行时](windows-svn-runtime.md)。不修改用户的 SVN 安装或系统区域设置。

根因：旧 Windows SVN 在命令处理之前调用 WideCharToMultiByte(CP_ACP, ...)；设置 LC_ALL 无法消除这个有损转换。应用已经使用 CREATE_NO_WINDOW；终端脚本继承控制台时可能产生额外乱码，因此不能把终端结果直接当作桌面端结果。诊断脚本仅作为原始 CLI 基线，保留 stderr 原始字节供排查。

新的真实仓库回归要求中文与 emoji 工作区、中文/日文/emoji/全角文件名、Unicode 提交消息全部成功，覆盖 checkout/add/commit/info/diff/revert/log。默认内置入口与 PATH 中旧客户端的兼容执行均通过。原 Windows 跳过的中文部分提交、文本 MIME 与嵌套未跟踪差异测试已恢复并通过。数值代码页检查覆盖 1252、936 与 UTF-8。

Windows x64/ARM64/x86 运行时归档均可成功准备；ARM64/x86 尚未执行原生运行测试。英语 Windows 远程 CI 尚未运行，不能把本机 ANSI 936 的结果表述成远程通过。远程 Issue 待代码合入发布后关闭。

## 仍需验收

- 真实 NSIS/MSI 构建、安装/卸载 PATH、文件占用及 UAC 接受/取消。
- 日志日期筛选的深色和窄屏截图、长路径差异页原始复现条件。
- 编辑器发现、自定义路径、拖动及原生文件拖放行为。
- macOS / Linux 安装包与命令入口实机验证。
- 提交树虚拟化保留原待办的暂缓状态；本次未更改该视图。

