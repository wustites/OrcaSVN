# OrcaSVN 快速使用

## 打开工作副本

点击“打开工作区”，选择包含 `.svn` 元数据的目录。OrcaSVN 会同时读取 `svn status` 和 `svn info`。

也可以使用 Checkout，从仓库 URL 创建新的工作副本。

## 理解工作区状态

工作区借鉴 `git status` 的信息层级，但操作仍遵循 SVN 语义：

| 分类 | SVN 状态 | 建议操作 |
| --- | --- | --- |
| 变更 | modified、added、deleted、replaced | 查看 Diff 后提交或还原 |
| 未版本 | unversioned | 确认后执行 Add |
| 冲突 | conflicted | 解决内容后执行 Resolve |
| 缺失 | missing | 恢复文件，或标记为 Delete |

## 推荐工作流

1. 执行 Update 获取远端最新修改。
2. 处理冲突和缺失文件。
3. 查看每个文件的 Diff。
4. 只选择本次相关文件，填写清楚的提交信息。
5. Commit 后刷新工作区，确认没有意外残留。

## 常见操作

- **Diff**：查看工作副本与基准版本的差异。
- **Log**：浏览仓库提交历史和涉及路径。
- **Blame**：查看每一行最后一次修改的版本和作者。
- **Revert**：永久丢弃本地修改，操作前请确认 Diff。
- **Cleanup**：工作副本被锁定或操作中断时使用。
- **Switch**：将当前工作副本切换到另一个仓库 URL。

## 命令行对应关系

OrcaSVN 的主要操作与 SVN CLI 一一对应：

```text
打开工作区  -> svn status --xml + svn info --xml
更新        -> svn update
提交        -> svn commit
差异        -> svn diff
历史        -> svn log --xml --verbose
注解        -> svn blame
```
