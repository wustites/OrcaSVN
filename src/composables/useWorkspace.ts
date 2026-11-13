import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo, svnLocalRevision, readGitignore, workspaceLatestModified } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { useSettings } from '@/composables/useSettings'
import i18n from '@/i18n'
import { parseGitignore } from '@/utils/gitignore'
import { filterByGitignoreAsync } from '@/utils/gitignoreWorker'
import { cacheSvnInfoMetadata, getCachedSvnInfoMetadata, type SvnInfoMetadata } from '@/utils/svnInfoCache'
import { cacheWorkspaceSnapshot, getWorkspaceSnapshot, invalidateWorkspaceSnapshot } from '@/utils/workspaceSnapshot'
import type { SvnInfo, SvnStatus } from '@/types'

let workspaceRequestGeneration = 0
let pendingRequest: { path: string; generation: number; force: boolean; background: boolean; promise: Promise<boolean> } | null = null
// 后台轮询只报告一次相同的状态错误，恢复时不清除其他操作产生的错误。
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

  async function updateSummary(path: string, status: SvnStatus[], request: number) {
    // 复用本次状态扫描的文件时间缓存，不为当前工作区重复执行 SVN 扫描。
    const modified = await workspaceLatestModified(path).catch(() => null)
    workspaceStore.setWorkspaceSummary(path, status, modified, request)
  }

  async function performLoadWorkspace(path: string): Promise<boolean> {
    const generation = ++workspaceRequestGeneration
    const summaryRequest = workspaceStore.beginWorkspaceSummary(path)
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
      void updateSummary(path, filteredStatus, summaryRequest)
      // 状态扫描成功即可使用工作区；版本和仓库信息读取失败只提示，不关闭工作区。
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
      workspaceStore.setWorkspaceSummaryError(path, `读取工作区失败：${String(err)}`, summaryRequest)

      if (snapshot) {
        workspaceStore.setStatusList(snapshot.statusList)
        workspaceStore.setSvnInfo(snapshot.svnInfo)
        workspaceStore.setStatusIsStale(true)
        reportStatusError(path, String(err))
        return false
      }
      // 切换期间被移除的旧工作区不能因目标加载失败而重新出现。
      if (previousPath && workspaceStore.recentWorkspaces.includes(previousPath)) {
        workspaceStore.setCurrentPath(previousPath, false)
        workspaceStore.setStatusList(previousStatusList)
        workspaceStore.setSvnInfo(previousSvnInfo)
        workspaceStore.setStatusIsStale(previousStatusIsStale)
      } else {
        workspaceStore.resetWorkspaceState()
      }
      reportStatusError(path, String(err))
      return false
    } finally {
      if (isCurrentGeneration()) workspaceStore.setLoading(false)
    }
  }

  async function openWorkspace(selectDialogTitle: string): Promise<boolean> {
    const selected = await open({
      directory: true,
      multiple: true,
      title: selectDialogTitle,
    })

    if (!selected) return false

    const paths = workspaceStore.registerWorkspaces(Array.isArray(selected) ? selected : [selected])
    if (!paths.length) return false
    // 一次加入全部目录，后台串行读取摘要；已有工作区保持不变。
    if (workspaceStore.currentPath) return true
    return loadWorkspace(paths[0])
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
    const summaryRequest = workspaceStore.beginWorkspaceSummary(path)
    const isCurrent = () => (
      generation === workspaceRequestGeneration && workspaceStore.currentPath === path
    )

    workspaceStore.setLoading(true)
    if (!background) clearStatusError()
    let statusVerified = false
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
      statusVerified = true
      void updateSummary(path, filteredStatus, summaryRequest)
      // 保留刚扫描的状态和摘要，同时向调用方报告版本信息未刷新。
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
      if (!statusVerified) workspaceStore.setWorkspaceSummaryError(path, `读取工作区失败：${message}`, summaryRequest)
      if (!background || message !== lastStatusError?.message) reportStatusError(path, message)
      return false
    } finally {
      if (isCurrent()) workspaceStore.setLoading(false)
    }
  }

  function loadWorkspace(path: string): Promise<boolean> {
    // 沿用已有记录的路径写法，确保选中状态、摘要和去重请求使用同一个键。
    path = workspaceStore.resolveWorkspacePath(path)
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
