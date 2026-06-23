# OrcaSVN 性能优化记录

## 1. 日志查询（LogView）—— 过滤优化

### 优化：客户端短路求值 + 服务端过滤下推

#### 修改文件
- `src/views/LogView.vue`
- `src/api/svn.ts`
- `src-tauri/src/svn/operations.rs`
- `src-tauri/src/main.rs`

#### 客户端：`filteredLogs` 短路求值

**之前**：每次 `filteredLogs` 重新计算时，无论是否有过滤条件，都遍历全部 `logs.value`，每条执行 `new Date()` + `toLowerCase()` + 字符串拼接。

**之后**：
```
无过滤条件 → 直接返回 logs.value，零遍历开销
有过滤条件 → 按 作者→关键字→日期 顺序早返回，每个条件只在需要时计算
```

具体优化点：

| 场景 | 优化前 | 优化后 |
|------|--------|--------|
| 无过滤条件 | 仍遍历全部 `logs.value` | `if (!author && !keyword && !from && !to) return logs.value` |
| 有关键字 | 每条都构造 searchable 字符串 | `if (keyword) { /* 按需构造 */ }` |
| 有日期范围 | 每条都 `new Date(entry.date)` | `if (from \|\| to) { /* 按需解析 */ }` |
| 作者不匹配 | 进入 return 判断 | 提前 `return false` 跳过后续计算 |

#### 服务端：过滤条件下推到 SVN

将前端过滤条件（关键字、日期范围）传递给 `svn log` 命令参数，让 SVN 服务器端进行过滤，减少网络传输和解析量。

| 场景 | 优化前 | 优化后 |
|------|--------|--------|
| 有关键字过滤 | 拉取全部日志 → 前端逐条匹配 | `svn log --search "keyword"` |
| 有日期范围 | 拉取全部日志 → 前端 Date 比较 | `svn log -r {2024-01-01}:{2024-03-01}` |
| 两者都有 | 同上 | `--search "bug" -r {from}:{to}` 双重过滤 |

**后端逻辑**（`operations.rs` `log()` 函数）：
- 当 `date_from` 存在时，优先构建 `-r {date_from}:{date_to}` 日期范围参数
- 当 `keyword` 非空时，添加 `--search keyword` 参数
- load-more 场景兼容：日期范围 + 修订号上界 `-r {date_from}:{oldest_rev - 1}`

**缓存适配**：缓存 Key 加入 `keyword`/`dateFrom`/`dateTo`，不同过滤条件的查询结果独立缓存。

---

## 2. SVG 日期选择器不显示（Bug 修复）

### 问题
`<el-date-picker>` 组件从未被注册，导致整个日期选择器元素不渲染。

### 修改文件
- `src/main.ts`

### 修改内容
添加 `ElDatePicker` 的导入和注册：
```ts
import { ElDatePicker } from 'element-plus/es/components/date-picker/index'

const components = [
  // ...
  ElDatePicker,
  // ...
]
```

---

## 3. 日志缓存优化

### 修改文件
- `src/views/LogView.vue`

### 3.1 缓存 Key 免 JSON 解析

**之前**：缓存 Key 使用 `JSON.stringify({ path, limit, startRev, endRev })`，`getCachedAuthorsForPath` 和 `clearLogCacheForPath` 需要逐条 `JSON.parse` 解析。

**之后**：缓存 Key 改为纯字符串拼接：
```
${path}|${limit}|${startRev ?? ''}|${endRev ?? ''}|${keyword ?? ''}|${dateFrom ?? ''}|${dateTo ?? ''}
```
遍历检查改为 `key.startsWith(prefix)`，消除 O(n) 的 JSON 解析开销。

### 3.2 缓存大小限制

新增 `MAX_CACHE_SIZE = 50` 上限，`setCachedLogPage` 超出时自动淘汰最旧条目，防止内存泄漏。

---

## 4. 初始化性能优化

### 修改文件
- `src/views/LogView.vue`
- `src/composables/useWorkspace.ts`
- `src/layouts/MainLayout.vue`

### 4.1 消除 `svn log` 重复调用（LogView）

**之前**：`onMounted` + `watch(currentPath)` 两处都可能触发 `reloadLogs()`，初始化时可能发出 2 次 `svn log`。

**之后**：移除 `onMounted`/`onActivated` 的 `reloadLogs()` 调用，`watch(currentPath)` 改为 `{ immediate: true }` 单点触发。

### 4.2 三路并行（useWorkspace）

**之前**：`setCurrentPath(path)` 在 `svnStatus` + `svnInfo` 异步完成后才执行，LogView 的 `svn log` 被迫串行等待。
```
status ──→ info ──→ setPath ──→ log
```

**之后**：`setCurrentPath(path)` 提前到异步操作开始前执行，LogView 的 `immediate` watch 立即触发 `svn log`，实现三路并行：
```
status ──┐
info ────┤  (同时进行)
log ─────┘
```

### 4.3 轮询间隔延长

**之前**：`svn status --xml` 每 10 秒轮询一次。

**之后**：间隔改为 30 秒，减少 2/3 的磁盘 I/O。定位在 `src/layouts/MainLayout.vue:182`。

---

## 修改文件汇总

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `src/views/LogView.vue` | 性能优化 + Bug 修复 | filteredLogs 短路求值、缓存 Key/大小优化、初始化去重、服务端过滤参数传递 |
| `src/api/svn.ts` | 接口扩展 | `svnLog()` 新增 keyword/dateFrom/dateTo 参数 |
| `src/main.ts` | Bug 修复 | 注册缺失的 `ElDatePicker` 组件 |
| `src/composables/useWorkspace.ts` | 性能优化 | `setCurrentPath` 提前，实现三路并行 |
| `src/layouts/MainLayout.vue` | 性能优化 | 状态轮询间隔 10s → 30s |
| `src-tauri/src/main.rs` | 接口扩展 | `svn_log` Tauri 命令新增 keyword/date_from/date_to |
| `src-tauri/src/svn/operations.rs` | 功能增强 | `log()` 支持 `--search` 和日期范围 `-r {date}:{date}` |
