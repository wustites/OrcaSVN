# OrcaSVN 快速开始

## 项目已创建完成！

这是一个基于 **Tauri + Vue 3 + TypeScript** 的 SVN 图形客户端。

## 项目结构

```
OrcaSVN/
├── src/                    # 前端源代码
│   ├── api/               # SVN API 调用封装
│   ├── layouts/           # 主布局组件
│   ├── router/            # 路由配置
│   ├── stores/            # Pinia 状态管理
│   ├── types/             # TypeScript 类型定义
│   ├── views/             # 页面组件
│   │   ├── WorkspaceView.vue   # 工作区
│   │   ├── CheckoutView.vue    # Checkout 功能
│   │   ├── CommitView.vue      # Commit 功能
│   │   ├── LogView.vue         # 日志查看
│   │   ├── DiffView.vue        # 代码对比
│   │   ├── BlameView.vue       # 文件注解
│   │   └── SettingsView.vue    # 设置
│   ├── App.vue
│   └── main.ts
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── main.rs        # Tauri 命令处理
│   │   └── svn.rs         # SVN 命令实现
│   └── Cargo.toml
└── package.json
```

## 功能列表

| 功能 | 说明 | 状态 |
|------|------|------|
| Checkout | 从 SVN 仓库检出代码 | ✅ |
| Update | 更新工作副本 | ✅ |
| Commit | 提交更改 | ✅ |
| Status | 查看文件状态 | ✅ |
| Log | 查看提交历史 | ✅ |
| Diff | 代码对比 | ✅ |
| Blame | 文件注解 | ✅ |
| Add | 添加文件 | ✅ |
| Delete | 删除文件 | ✅ |
| Revert | 还原更改 | ✅ |
| Cleanup | 清理工作区 | ✅ |
| Switch | 切换分支/URL | ✅ |
| Merge | 合并 | ✅ |

## 下一步

### 1. 安装 Rust

项目需要 Rust 环境才能运行。访问 https://www.rust-lang.org/tools/install 安装。

### 2. 安装依赖

```bash
npm install
```

### 3. 运行开发模式

```bash
npm run tauri dev
```

### 4. 构建发布版本

```bash
npm run tauri build
```

## 使用说明

1. **打开工作区**: 点击"打开工作区"按钮，选择一个已有的 SVN 工作目录
2. **Checkout**: 从 SVN 仓库检出代码到新目录
3. **查看状态**: 在工作区查看文件修改状态
4. **提交更改**: 选择文件并输入提交信息进行提交
5. **代码对比**: 查看工作副本与基准版本的差异
6. **查看注解**: 查看每行代码的最后修改者和版本

## 技术特点

- 🚀 **现代化 UI**: 使用 Element Plus 组件库
- 🔒 **安全可靠**: Tauri 提供系统级安全保护
- ⚡ **高性能**: Rust 后端处理 SVN 命令
- 📦 **跨平台**: 支持 Windows、macOS、Linux
- 🎨 **响应式**: 适配不同屏幕尺寸
