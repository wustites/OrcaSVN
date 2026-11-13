# README 截图演示数据

截图使用固定随机种子生成的 10 个虚构工作区，其中 3 个为收藏。项目名、备注、文件状态、Changelist、时间、版本号和代码差异均为演示内容。

- 工作区目录统一为虚构的 `C:\Demo\Workspaces\...`，不会创建或读取这些目录。
- 仓库地址统一使用 `svn.example.test`，不会连接 SVN 服务。
- 数据文件：[workspace-demo-data.json](images/workspace-demo-data.json)。
- 截图脚本：[generate-readme-screenshots.cjs](../scripts/generate-readme-screenshots.cjs)。

脚本自动启动并关闭独立的本地 Vite 服务，以全新浏览器上下文模拟 Tauri 调用，阻断外部网络请求，不读取桌面程序或日常浏览器的配置、账户及工作区记录。

## 重新生成

安装项目依赖和截图工具后，在项目根目录运行：

```bash
npm ci
npm install --no-save --package-lock=false playwright
node scripts/generate-readme-screenshots.cjs
```

Windows 默认使用已安装的 Microsoft Edge；其他系统需先运行 `npx playwright install chromium`。可用环境变量 `SCREENSHOT_BROWSER_CHANNEL` 指定 Playwright 支持的浏览器通道。

生成的文件位于 `docs/images/`：

- `orcasvn-workspace.png`：全部工作区、文件分组和差异预览。
- `orcasvn-workspace-favorites.png`：收藏工作区筛选。
- `orcasvn-workspace-dark.png`：深色主题。
- `workspace-demo-data.json`：上述截图所用的虚构数据。

这些演示数据仅供文档截图使用，不是应用的默认配置，也不会写入正式程序的数据目录。
