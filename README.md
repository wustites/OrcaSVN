# OrcaSVN

基于 Tauri 的 SVN 图形客户端，使用 Rust + TypeScript + Vue 3 开发。

## 功能特性

- ✅ **Checkout** - 从 SVN 仓库检出代码
- ✅ **Commit** - 提交更改到仓库
- ✅ **Update** - 更新工作副本
- ✅ **Status** - 查看文件状态
- ✅ **Log** - 查看提交历史
- ✅ **Diff** - 代码对比
- ✅ **Blame** - 文件注解
- ✅ **Add/Delete/Revert** - 文件操作
- ✅ **Cleanup** - 清理工作区
- ✅ **Switch** - 切换分支/URL

## 技术栈

- **前端**: Vue 3 + TypeScript + Vite
- **UI 框架**: Element Plus
- **状态管理**: Pinia
- **后端**: Rust + Tauri
- **SVN 交互**: 调用 svn 命令行工具

## 开发环境要求

1. [Node.js](https://nodejs.org/) (v18 或更高版本)
2. [Rust](https://www.rust-lang.org/tools/install) (最新稳定版)
3. [SVN](https://subversion.apache.org/packages.html) 命令行工具

### Windows 额外要求

- Microsoft Visual Studio C++ Build Tools
- WebView2 (Windows 10 1803+ 已内置)

## 安装依赖

```bash
npm install
```

## 开发模式

```bash
npm run tauri dev
```

## 构建应用

```bash
npm run tauri build
```

## 项目结构

```
OrcaSVN/
├── src/                    # 前端源代码
│   ├── api/               # API 调用
│   ├── layouts/           # 布局组件
│   ├── router/            # 路由配置
│   ├── stores/            # Pinia 状态管理
│   ├── types/             # TypeScript 类型定义
│   ├── views/             # 页面组件
│   ├── App.vue            # 根组件
│   ├── main.ts            # 入口文件
│   └── style.css          # 全局样式
├── src-tauri/             # Tauri/Rust 后端
│   ├── src/
│   │   ├── main.rs        # Rust 主入口
│   │   └── svn.rs         # SVN 命令实现
│   ├── Cargo.toml         # Rust 依赖配置
│   ├── build.rs           # 构建脚本
│   └── tauri.conf.json    # Tauri 配置
├── package.json           # Node.js 依赖配置
├── tsconfig.json          # TypeScript 配置
├── vite.config.ts         # Vite 配置
└── index.html             # HTML 模板
```

## 使用说明

1. 启动应用后，点击"打开工作区"选择一个 SVN 工作目录
2. 或者使用"Checkout"功能从仓库检出代码
3. 在工作区可以查看文件状态、提交更改、查看日志等
4. 使用"对比"功能查看代码差异
5. 使用"注解"功能查看每行代码的最后修改者

## 许可证

MIT License
