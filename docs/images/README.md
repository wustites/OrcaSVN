# 文档截图

更新日期：2026-11-13。截图使用当前 Vue 界面，在独立的本地 Vite 页面中渲染，不包含原生窗口边框。所有项目、路径、备注、时间、文件状态和代码均为虚构数据，没有读取本机项目或连接 SVN 服务。

| 文件 | 场景 |
| --- | --- |
| `orcasvn-workspace.png` | 简体中文、浅色主题、1600 × 960，全部工作区、Changelist 分组及 Diff |
| `orcasvn-workspace-favorites.png` | 同一场景下的收藏工作区筛选 |
| `orcasvn-workspace-dark.png` | 全部工作区的深色主题 |
| `workspace-demo-data.json` | 截图使用的虚构数据，固定随机种子为 `20261113` |

生成方式和隔离机制见[截图演示说明](../screenshot-demo.md)。在安装依赖后运行：

```bash
node scripts/generate-readme-screenshots.cjs
```

该脚本在应用加载前注入模拟命令，使用全新浏览器上下文，阻断外部请求，并在完成后关闭浏览器和预览服务。演示数据仅用于截图，不会作为正式程序的默认工作区。
