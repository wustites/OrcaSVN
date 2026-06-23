import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'

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

      workspaceStore.setStatusList(status)
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
      workspaceStore.setStatusList(status)
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
