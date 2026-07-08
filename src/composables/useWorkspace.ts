import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo, readGitignore } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { useSettings } from '@/composables/useSettings'
import { parseGitignore, isIgnored } from '@/utils/gitignore'
import type { GitignorePattern } from '@/utils/gitignore'
import type { SvnStatus } from '@/types'

async function loadGitignoreIfNeeded(path: string, store: ReturnType<typeof useWorkspaceStore>): Promise<void> {
  const { settings } = useSettings()
  if (!settings.gitignoreEnabled) {
    store.setGitignorePatterns([])
    store.setGitignoreMtime(null)
    return
  }

  try {
    const data = await readGitignore(path)
    if (!data) {
      store.setGitignorePatterns([])
      store.setGitignoreMtime(null)
      return
    }

    if (data.mtime === store.gitignoreMtime && store.gitignorePatterns.length > 0) return

    const patterns = parseGitignore(data.content)
    store.setGitignorePatterns(patterns)
    store.setGitignoreMtime(data.mtime)
  } catch {
    store.setGitignorePatterns([])
    store.setGitignoreMtime(null)
  }
}

function filterByGitignore(
  list: SvnStatus[],
  patterns: GitignorePattern[]
): SvnStatus[] {
  if (patterns.length === 0) return list
  return list.filter(s => !isIgnored(s.path, patterns))
}

export function useWorkspace() {
  const workspaceStore = useWorkspaceStore()

  async function loadWorkspace(path: string): Promise<boolean> {
    workspaceStore.setLoading(true)
    workspaceStore.setError(null)

    // 提前设置 currentPath，让 LogView 的 svn log 与 status+info 并行执行
    workspaceStore.setCurrentPath(path)

    try {
      const [status, info] = await Promise.all([
        svnStatus(path),
        svnInfo(path),
      ])

      await loadGitignoreIfNeeded(path, workspaceStore)
      workspaceStore.setStatusList(filterByGitignore(status, workspaceStore.gitignorePatterns))
      workspaceStore.setSvnInfo(info)
      return true
    } catch (err) {
      workspaceStore.setError(String(err))
      return false
    } finally {
      workspaceStore.setLoading(false)
    }
  }

  async function openWorkspace(selectDialogTitle: string): Promise<boolean> {
    const selected = await open({
      directory: true,
      multiple: false,
      title: selectDialogTitle,
    })

    if (!selected) return false

    const path = Array.isArray(selected) ? selected[0] : selected
    return loadWorkspace(path)
  }

  async function restoreLastWorkspace(): Promise<boolean> {
    const path = workspaceStore.getLastWorkspacePath()
    if (!path) return false
    return loadWorkspace(path)
  }

  async function refreshStatus(): Promise<boolean> {
    if (!workspaceStore.currentPath) return false

    workspaceStore.setLoading(true)
    workspaceStore.setError(null)
    try {
      const [status, info] = await Promise.all([
        svnStatus(workspaceStore.currentPath),
        svnInfo(workspaceStore.currentPath),
      ])

      await loadGitignoreIfNeeded(workspaceStore.currentPath, workspaceStore)
      workspaceStore.setStatusList(filterByGitignore(status, workspaceStore.gitignorePatterns))
      workspaceStore.setSvnInfo(info)
      return true
    } catch (err) {
      workspaceStore.setError(String(err))
      return false
    } finally {
      workspaceStore.setLoading(false)
    }
  }

  return { openWorkspace, restoreLastWorkspace, refreshStatus }
}
