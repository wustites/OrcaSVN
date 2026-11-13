import { onUnmounted, ref, watch } from 'vue'
import { readGitignore, svnStatus, workspaceLatestModified } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useSettings } from '@/composables/useSettings'
import { useWorkspace } from '@/composables/useWorkspace'
import { parseGitignore } from '@/utils/gitignore'
import { filterByGitignoreAsync } from '@/utils/gitignoreWorker'

export function useWorkspaceSummaries() {
  const store = useWorkspaceStore()
  const { settings } = useSettings()
  const { refreshStatusIfStale } = useWorkspace()
  const refreshing = ref(false)
  const queued = new Set<string>()
  let disposed = false
  let pending: Promise<void> | null = null
  let refreshCurrent = false

  async function refreshOne(path: string) {
    if (!store.recentWorkspaces.includes(path) || path === store.currentPath) return
    const request = store.beginWorkspaceSummary(path)
    const gitignoreEnabled = settings.gitignoreEnabled
    const isCurrent = () => !disposed && store.isWorkspaceSummaryCurrent(path, request)
      && path !== store.currentPath && settings.gitignoreEnabled === gitignoreEnabled
    try {
      const status = await svnStatus(path, false)
      if (!isCurrent()) return
      const modified = await workspaceLatestModified(path).catch(() => null)
      let patterns: ReturnType<typeof parseGitignore> = []
      if (gitignoreEnabled) {
        const data = await readGitignore(path)
        if (data) patterns = parseGitignore(data.content)
      }
      const filtered = await filterByGitignoreAsync(status, patterns)
      if (isCurrent()) store.setWorkspaceSummary(path, filtered, modified, request)
    } catch (error) {
      if (isCurrent()) store.setWorkspaceSummaryError(path, `读取工作区失败：${String(error)}`, request)
    }
  }

  async function drain() {
    refreshing.value = true
    try {
      while (!disposed && (refreshCurrent || queued.size > 0)) {
        if (refreshCurrent) {
          refreshCurrent = false
          // 当前工作区优先，复用正在执行的加载或刷新任务。
          if (store.currentPath) await refreshStatusIfStale(0)
        }
        const path = queued.values().next().value as string | undefined
        if (!path) continue
        queued.delete(path)
        // 其余项目串行读取，避免多项目同时遍历磁盘。
        await refreshOne(path)
      }
    } finally {
      refreshing.value = false
      pending = null
    }
  }

  function start(): Promise<void> {
    if (disposed) return Promise.resolve()
    if (!pending) pending = Promise.resolve().then(drain)
    return pending
  }

  function refreshSummaries(): Promise<void> {
    if (disposed) return Promise.resolve()
    refreshCurrent = true
    for (const path of store.recentWorkspaces) queued.add(path)
    return start()
  }

  const stopPaths = watch(() => [...store.recentWorkspaces], (paths, previous) => {
    for (const path of paths) {
      if (!previous.includes(path) && path !== store.currentPath) queued.add(path)
    }
    if (queued.size) void start()
  })
  const stopSettings = watch(() => settings.gitignoreEnabled, () => {
    void refreshSummaries()
  })

  onUnmounted(() => {
    disposed = true
    queued.clear()
    stopPaths()
    stopSettings()
  })

  return { refreshSummaries, refreshing }
}
