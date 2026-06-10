# Contributing

感谢参与 OrcaSVN。请保持改动聚焦、可验证，并确保界面语义与 SVN CLI 行为一致。

## 开发流程

1. 从 `main` 创建短生命周期分支。
2. 使用 `npm ci` 安装锁定版本的依赖。
3. 实现改动，并为解析、参数构造等纯逻辑补充测试。
4. 运行 `npm run check`。
5. 在 Pull Request 中说明用户可见行为、风险和验证方式。

## 代码约定

- 前端使用 Vue 3 Composition API、TypeScript 和现有 Pinia/composable 模式。
- 后端保持 `executor`、`operations`、`parser` 的职责边界。
- 文件路径参数必须放在 SVN 的 `--` 参数边界之后。
- 新增用户可见文本时同步更新全部语言资源。
- 不提交 `dist/`、`node_modules/` 或 `src-tauri/target/`。

## 提交信息

推荐使用简洁的 Conventional Commits 风格：

```text
feat: add conflict status filter
fix: separate svn file targets from options
docs: clarify local development workflow
```
