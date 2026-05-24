# OrcaSVN 优化任务清单

## P0 - 必须修复 ✅

- [x] 清除所有 `console.log` 调试语句 (23处)
- [x] 删除 `stores/workspace.ts` 中重复的类型定义，统一引用 `types/index.ts`
- [x] 修复硬编码中文字符串，替换为 i18n key
- [x] 设置 Tauri CSP 策略 (`tauri.conf.json` 中 `"csp": null`)
- [x] 实现或移除 `select_directory` 空函数

## P1 - 重要优化 ✅

- [x] 修复 Store 封装性：所有 `workspaceStore.xxx = yyy` 改为 action 调用
- [x] 提取重复代码为 composable/utils
- [x] 引入 `quick-xml` 替代正则解析 XML
- [x] 优化 blame 双重 SVN 调用 → 直接使用纯文本格式解析
- [x] 移除 `--trust-server-cert` 硬编码
- [x] 替换 `eprintln!` 为 `log` crate
- [x] 统一错误映射风格 (match vs map_err)
- [x] 缩小 tokio features: `"full"` → `["rt", "time"]`

## P2 - 功能完善 ✅

- [x] 路由懒加载：所有视图组件改为 `() => import(...)`
- [x] 设置持久化 (localStorage)
- [x] 语言偏好持久化
- [x] 实现暗色主题切换
- [x] 实现 `doAdd` / `doDelete` / `doSwitch` 功能
- [x] 添加路由守卫和 404 兜底
- [x] 添加 SVN 操作超时机制 (120秒)
- [x] `formatDate` 跟随当前语言

## P3 - 清理与测试 ✅

- [x] 移除未使用的依赖 `@material/web`
- [x] 移除未使用的依赖 `serde_json` (Cargo.toml)
- [x] 为解析函数添加单元测试 (8个测试用例)
- [x] 添加结构化日志框架 (`tracing`)
- [x] 拆分 `svn.rs` 为子模块 (executor/parser/operations)
- [x] 补充路由 meta 信息
