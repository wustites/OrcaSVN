import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo, svnLocalRevision, readGitignore } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { useSettings } from '@/composables/useSettings'
import { parseGitignore, isIgnored } from '@/utils/gitignore'
import { cacheSvnInfoMetadata, getCachedSvnInfoMetadata, type SvnInfoMetadata } from '@/utils/svnInfoCache'
import type { GitignorePattern } from '@/utils/gitignore'
import type { SvnInfo, SvnStatus } from '@/types'

let workspaceRequestGeneration = 0

async function loadGitignoreIfNeeded(
  path: string,
  store: ReturnType<typeof useWorkspaceStore>,
  isCurrent: () => boolean,
): Promise<void> {
  const { settings } = useSettings()
  if (!settings.gitignoreEnabled) {
    if (!isCurrent()) return
    store.setGitignorePatterns([])
    store.setGitignoreMtime(null)
    store.setGitignoreWorkspacePath(null)
    return
  }

  try {
    const data = await readGitignore(path)
    if (!isCurrent()) return
    if (!data) {
      store.setGitignorePatterns([])
      store.setGitignoreMtime(null)
      store.setGitignoreWorkspacePath(path)
      return
    }

    if (
      path === store.gitignoreWorkspacePath &&
      data.mtime === store.gitignoreMtime
    ) return

    const patterns = parseGitignore(data.content)
    if (!isCurrent()) return
    store.setGitignorePatterns(patterns)
    store.setGitignoreMtime(data.mtime)
    store.setGitignoreWorkspacePath(path)
  } catch {
    if (!isCurrent()) return
    store.setGitignorePatterns([])
    store.setGitignoreMtime(null)
    store.setGitignoreWorkspacePath(null)
  }
}

function filterByGitignore(
  list: SvnStatus[],
  patterns: GitignorePattern[]
): SvnStatus[] {
  if (patterns.length === 0) return list
  return list.filter(s => !isIgnored(s.path, patterns))
}

function toSvnInfoMetadata(info: SvnInfo): SvnInfoMetadata {
  const { revision: _revision, ...metadata } = info
  return metadata
}

async function loadWorkspaceInfo(path: string, isCurrent: () => boolean): Promise<SvnInfo | null> {
  const cachedMetadata = getCachedSvnInfoMetadata(path)
  const freshMetadataRequest = cachedMetadata ? null : svnInfo(path)
  const [revision, freshInfo] = await Promise.all([
    svnLocalRevision(path),
    freshMetadataRequest || Promise.resolve(null),
  ])

  if (!isCurrent()) return null

  const metadata = cachedMetadata || (freshInfo ? toSvnInfoMetadata(freshInfo) : null)
  if (!metadata) throw new Error('无法获取 SVN 工作区元信息')
  if (!cachedMetadata && freshInfo) cacheSvnInfoMetadata(path, metadata)

  return { ...metadata, revision }
}

type InfoRequestResult =
  | { ok: true; info: SvnInfo | null }
  | { ok: false; error: unknown }

function requestWorkspaceInfo(path: string, isCurrent: () => boolean): Promise<InfoRequestResult> {
  return loadWorkspaceInfo(path, isCurrent).then(
    info => ({ ok: true, info }),
    error => ({ ok: false, error }),
  )
}

export function useWorkspace() {
  const workspaceStore = useWorkspaceStore()

  async function loadWorkspace(path: string): Promise<boolean> {
    const generation = ++workspaceRequestGeneration
    const previousPath = workspaceStore.currentPath
    const previousStatusList = workspaceStore.statusList
    const previousSvnInfo = workspaceStore.svnInfo
    const isCurrentGeneration = () => generation === workspaceRequestGeneration
    const isCurrent = () => (
      isCurrentGeneration() && workspaceStore.currentPath === path
    )

    workspaceStore.setLoading(true)
    workspaceStore.setError(null)

    // 提前设置 currentPath，让依赖工作区路径的视图尽早切换。
    workspaceStore.setCurrentPath(path, false)
    workspaceStore.setStatusList([])
    workspaceStore.setSvnInfo(null)

    const statusRequest = svnStatus(path)
    // Convert the eager metadata request into a fulfilled result so a status
    // failure cannot leave a second rejected promise unobserved.
    const infoRequest = requestWorkspaceInfo(path, isCurrent)
    const gitignoreRequest = loadGitignoreIfNeeded(path, workspaceStore, isCurrent)

    try {
      // 变更列表不依赖仓库元数据，status 返回后立即展示。
      const status = await statusRequest
      if (!isCurrent()) return false
      workspaceStore.setStatusList(status)

      const [infoResult] = await Promise.all([infoRequest, gitignoreRequest])
      if (!infoResult.ok) throw infoResult.error
      const info = infoResult.info
      if (!isCurrent() || !info) return false
      workspaceStore.setStatusList(filterByGitignore(status, workspaceStore.gitignorePatterns))
      workspaceStore.setSvnInfo(info)
      workspaceStore.rememberWorkspace(path)
      return true
    } catch (err) {
      if (!isCurrent()) return false

      if (previousPath) workspaceStore.setCurrentPath(previousPath, false)
      else workspaceStore.clearWorkspace()
      workspaceStore.setStatusList(previousStatusList)
      workspaceStore.setSvnInfo(previousSvnInfo)
      workspaceStore.setError(String(err))
      return false
    } finally {
      if (isCurrentGeneration()) workspaceStore.setLoading(false)
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

    const generation = ++workspaceRequestGeneration
    const path = workspaceStore.currentPath
    const isCurrent = () => (
      generation === workspaceRequestGeneration && workspaceStore.currentPath === path
    )

    workspaceStore.setLoading(true)
    workspaceStore.setError(null)
    try {
      const statusRequest = svnStatus(path)
      const infoRequest = requestWorkspaceInfo(path, isCurrent)
      const gitignoreRequest = loadGitignoreIfNeeded(path, workspaceStore, isCurrent)
      const status = await statusRequest
      if (!isCurrent()) return false
      workspaceStore.setStatusList(status)
      const [infoResult] = await Promise.all([infoRequest, gitignoreRequest])
      if (!infoResult.ok) throw infoResult.error
      const info = infoResult.info
      if (!isCurrent() || !info) return false
      workspaceStore.setStatusList(filterByGitignore(status, workspaceStore.gitignorePatterns))
      workspaceStore.setSvnInfo(info)
      return true
    } catch (err) {
      if (!isCurrent()) return false
      workspaceStore.setError(String(err))
      return false
    } finally {
      if (isCurrent()) workspaceStore.setLoading(false)
    }
  }

  return { loadWorkspace, openWorkspace, restoreLastWorkspace, refreshStatus }
}
