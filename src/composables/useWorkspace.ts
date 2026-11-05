import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo, svnLocalRevision, readGitignore } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { useSettings } from '@/composables/useSettings'
import i18n from '@/i18n'
import { parseGitignore } from '@/utils/gitignore'
import { filterByGitignoreAsync } from '@/utils/gitignoreWorker'
import { cacheSvnInfoMetadata, getCachedSvnInfoMetadata, type SvnInfoMetadata } from '@/utils/svnInfoCache'
import { cacheWorkspaceSnapshot, getWorkspaceSnapshot, invalidateWorkspaceSnapshot } from '@/utils/workspaceSnapshot'
import type { SvnInfo } from '@/types'

let workspaceRequestGeneration = 0
let pendingRequest: { path: string; generation: number; force: boolean; background: boolean; promise: Promise<boolean> } | null = null
// The status failure last reported to the user, so that background polls report
// it once and clear it on recovery without touching errors from other operations.
let lastStatusError: { path: string; message: string } | null = null
let lastRefresh: { path: string; generation: number; timestamp: number } | null = null

function trackRequest(path: string, operation: () => Promise<boolean>, force = false, background = false): Promise<boolean> {
  const promise = operation()
  const request = { path, generation: workspaceRequestGeneration, force, background, promise }
  pendingRequest = request
  request.promise = promise.then(success => {
    if (success && request.generation === workspaceRequestGeneration) {
      lastRefresh = { path, generation: request.generation, timestamp: Date.now() }
    }
    return success
  }).finally(() => {
    if (pendingRequest === request) pendingRequest = null
  })
  return request.promise
}

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
  if (!metadata) throw new Error(i18n.global.t('workspace.svnInfoUnavailable'))
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

  function clearStatusError() {
    lastStatusError = null
    workspaceStore.setError(null)
  }

  function reportStatusError(path: string, message: string) {
    lastStatusError = { path, message }
    workspaceStore.setError(message)
  }

  async function performLoadWorkspace(path: string): Promise<boolean> {
    const generation = ++workspaceRequestGeneration
    const previousPath = workspaceStore.currentPath
    const previousStatusList = workspaceStore.statusList
    const previousSvnInfo = workspaceStore.svnInfo
    const previousStatusIsStale = workspaceStore.statusIsStale
    const { settings } = useSettings()
    const snapshot = getWorkspaceSnapshot(path, settings.gitignoreEnabled)
    const isCurrentGeneration = () => generation === workspaceRequestGeneration
    const isCurrent = () => (
      isCurrentGeneration() && workspaceStore.currentPath === path
    )

    workspaceStore.setLoading(true)
    clearStatusError()

    // 提前设置 currentPath，让依赖工作区路径的视图尽早切换。
    workspaceStore.setCurrentPath(path, false)
    workspaceStore.setStatusList(snapshot?.statusList || [])
    workspaceStore.setSvnInfo(snapshot?.svnInfo || null)
    workspaceStore.setStatusIsStale(Boolean(snapshot))

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
      if (!isCurrent() || (infoResult.ok && !infoResult.info)) return false
      const filteredStatus = await filterByGitignoreAsync(status, workspaceStore.gitignorePatterns)
      if (!isCurrent()) return false
      workspaceStore.setStatusList(filteredStatus)
      workspaceStore.setStatusIsStale(false)
      workspaceStore.rememberWorkspace(path)
      // The status scan succeeded, so the workspace is usable. Revision and
      // repository details are informational: report the failure but stay open.
      if (!infoResult.ok) {
        reportStatusError(path, String(infoResult.error))
        return true
      }
      if (infoResult.info) {
        workspaceStore.setSvnInfo(infoResult.info)
        cacheWorkspaceSnapshot(path, filteredStatus, infoResult.info, useSettings().settings.gitignoreEnabled)
      }
      return true
    } catch (err) {
      if (!isCurrent()) return false

      if (snapshot) {
        workspaceStore.setStatusList(snapshot.statusList)
        workspaceStore.setSvnInfo(snapshot.svnInfo)
        workspaceStore.setStatusIsStale(true)
        reportStatusError(path, String(err))
        return false
      }
      if (previousPath) workspaceStore.setCurrentPath(previousPath, false)
      else workspaceStore.resetWorkspaceState()
      workspaceStore.setStatusList(previousStatusList)
      workspaceStore.setSvnInfo(previousSvnInfo)
      workspaceStore.setStatusIsStale(previousStatusIsStale)
      reportStatusError(path, String(err))
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

  async function performRefreshStatus(force = false, background = false): Promise<boolean> {
    if (!workspaceStore.currentPath) return false

    const generation = ++workspaceRequestGeneration
    const path = workspaceStore.currentPath
    const isCurrent = () => (
      generation === workspaceRequestGeneration && workspaceStore.currentPath === path
    )

    workspaceStore.setLoading(true)
    if (!background) clearStatusError()
    try {
      const statusRequest = svnStatus(path, force)
      const infoRequest = requestWorkspaceInfo(path, isCurrent)
      const gitignoreRequest = loadGitignoreIfNeeded(path, workspaceStore, isCurrent)
      const status = await statusRequest
      if (!isCurrent()) return false
      workspaceStore.setStatusList(status)
      const [infoResult] = await Promise.all([infoRequest, gitignoreRequest])
      if (!isCurrent() || (infoResult.ok && !infoResult.info)) return false
      const filteredStatus = await filterByGitignoreAsync(status, workspaceStore.gitignorePatterns)
      if (!isCurrent()) return false
      workspaceStore.setStatusList(filteredStatus)
      workspaceStore.setStatusIsStale(false)
      // Keep the status just scanned; the caller learns the revision is not current.
      if (!infoResult.ok) throw infoResult.error
      if (infoResult.info) {
        workspaceStore.setSvnInfo(infoResult.info)
        cacheWorkspaceSnapshot(path, filteredStatus, infoResult.info, useSettings().settings.gitignoreEnabled)
      }
      if (lastStatusError?.path === path) {
        if (workspaceStore.error === lastStatusError.message) workspaceStore.setError(null)
        lastStatusError = null
      }
      return true
    } catch (err) {
      if (!isCurrent()) return false
      const message = String(err)
      if (!background || message !== lastStatusError?.message) reportStatusError(path, message)
      return false
    } finally {
      if (isCurrent()) workspaceStore.setLoading(false)
    }
  }

  function loadWorkspace(path: string): Promise<boolean> {
    if (pendingRequest?.path === path && !pendingRequest.background && pendingRequest.generation === workspaceRequestGeneration) return pendingRequest.promise
    return trackRequest(path, () => performLoadWorkspace(path))
  }

  function refreshStatus(): Promise<boolean> {
    const path = workspaceStore.currentPath
    if (!path) return Promise.resolve(false)
    if (pendingRequest?.path === path && pendingRequest.force && pendingRequest.generation === workspaceRequestGeneration) return pendingRequest.promise
    return trackRequest(path, () => performRefreshStatus(true), true)
  }

  // A mutation or setting change requires a scan started after that change;
  // an older background scan must not satisfy this request.
  function refreshStatusAfterMutation(): Promise<boolean> {
    const path = workspaceStore.currentPath
    if (path) {
      invalidateWorkspaceSnapshot(path)
      workspaceStore.setStatusIsStale(true)
    }
    return path ? trackRequest(path, () => performRefreshStatus(true), true) : Promise.resolve(false)
  }

  function refreshStatusIfStale(maxAgeMs: number): Promise<boolean> {
    const path = workspaceStore.currentPath
    if (!path) return Promise.resolve(false)
    if (pendingRequest?.path === path && pendingRequest.generation === workspaceRequestGeneration) return pendingRequest.promise
    if (lastRefresh?.path === path && lastRefresh.generation === workspaceRequestGeneration
      && Date.now() - lastRefresh.timestamp < maxAgeMs) return Promise.resolve(true)
    return trackRequest(path, () => performRefreshStatus(false, true), false, true)
  }

  // A failed launch path falls back to the last workspace; keep its error visible
  // because that fallback's own load would otherwise clear it.
  async function openStartupWorkspace(launchPath: string | null): Promise<void> {
    if (launchPath && await loadWorkspace(launchPath)) return
    const launchError = launchPath ? workspaceStore.error : null
    await restoreLastWorkspace()
    if (launchPath && launchError && !workspaceStore.error) reportStatusError(launchPath, launchError)
  }

  return { loadWorkspace, openWorkspace, restoreLastWorkspace, openStartupWorkspace, refreshStatus, refreshStatusAfterMutation, refreshStatusIfStale }
}
