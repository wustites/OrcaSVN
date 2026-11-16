import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { SvnStatus, SvnInfo } from '@/types'
import { invalidateWorkspaceSnapshot } from '@/utils/workspaceSnapshot'
import type { GitignorePattern } from '@/utils/gitignore'

const LAST_WORKSPACE_KEY = 'orcasvn-last-workspace'
const RECENT_WORKSPACES_KEY = 'orcasvn-recent-workspaces'
const FAVORITE_WORKSPACES_KEY = 'orcasvn-favorite-workspaces-v1'
const WORKSPACE_NOTES_KEY = 'orcasvn-workspace-notes-v1'
const WORKSPACE_SUMMARIES_KEY = 'orcasvn-workspace-summaries-v1'
const WORKSPACE_LIST_SETTINGS_KEY = 'orcasvn-workspace-list-settings-v1'

export type WorkspaceSortMode = 'manual' | 'name' | 'modified'

function readWorkspaceListSettings(): { sortMode: WorkspaceSortMode; collapsed: boolean } {
  try {
    const value = JSON.parse(localStorage.getItem(WORKSPACE_LIST_SETTINGS_KEY) || '{}')
    if (value && typeof value === 'object') {
      return {
        sortMode: ['manual', 'name', 'modified'].includes(value.sortMode) ? value.sortMode : 'manual',
        collapsed: value.collapsed === true,
      }
    }
  } catch { /* 损坏或不可读的偏好使用默认值。 */ }
  return { sortMode: 'manual', collapsed: false }
}

function workspaceIdentity(path: string): string {
  const normalized = path.replace(/\\/g, '/').replace(/\/+$/, '')
  return /^[a-z]:\//i.test(normalized) || normalized.startsWith('//') ? normalized.toLowerCase() : normalized
}

function writeStoredValues(entries: [string, string | null][], message: string) {
  const previous: [string, string | null][] = []
  try {
    for (const [key, value] of entries) {
      previous.push([key, localStorage.getItem(key)])
      if (value === null) localStorage.removeItem(key)
      else localStorage.setItem(key, value)
    }
  } catch {
    // 多项保存中途失败时回滚已写入的数据，不显示未保存的顺序。
    for (const [key, value] of previous.reverse()) {
      try {
        if (value === null) localStorage.removeItem(key)
        else localStorage.setItem(key, value)
      } catch { /* 存储不可用时仍保留内存中的原状态，并向用户报告。 */ }
    }
    throw new Error(message)
  }
}

export interface WorkspaceSummary {
  changed: number
  missing: number
  latestModified: number | null
  scannedAt: number
  error?: string
}

function readRecord<T>(key: string, validate: (value: unknown) => value is T): Record<string, T> {
  try {
    const stored = JSON.parse(localStorage.getItem(key) || '{}')
    if (stored && typeof stored === 'object' && !Array.isArray(stored)) {
      return Object.fromEntries(Object.entries(stored).filter((entry): entry is [string, T] => validate(entry[1])))
    }
  } catch { /* 忽略损坏的旧配置，工作区仍可正常打开。 */ }
  return {}
}

function isSummary(value: unknown): value is WorkspaceSummary {
  if (!value || typeof value !== 'object') return false
  const summary = value as WorkspaceSummary
  return [summary.changed, summary.missing].every(count => Number.isInteger(count) && count >= 0)
    && Number.isFinite(summary.scannedAt) && summary.scannedAt > 0
    && (summary.latestModified === null || (Number.isFinite(summary.latestModified) && summary.latestModified >= 0))
    && (summary.error === undefined || typeof summary.error === 'string')
}

function readRecentWorkspaces(): string[] {
  try {
    const stored = JSON.parse(localStorage.getItem(RECENT_WORKSPACES_KEY) || '[]')
    if (Array.isArray(stored)) {
      return [...new Set(stored.filter((path): path is string => typeof path === 'string' && path.length > 0))]
    }
  } catch {
    // 忽略旧安装或手动写入的损坏数据。
  }
  return []
}

function readFavoriteWorkspaces(paths: string[]): string[] {
  try {
    const stored = JSON.parse(localStorage.getItem(FAVORITE_WORKSPACES_KEY) || '[]')
    if (Array.isArray(stored)) {
      const savedPaths = new Map<string, string>()
      for (const path of paths) {
        const identity = workspaceIdentity(path)
        if (!savedPaths.has(identity)) savedPaths.set(identity, path)
      }
      const favorites = stored
        .filter((path): path is string => typeof path === 'string')
        .map(path => savedPaths.get(workspaceIdentity(path)))
        .filter((path): path is string => path !== undefined)
      return [...new Set(favorites)]
    }
  } catch { /* 损坏或不可读的收藏记录使用空列表，不影响全部工作区。 */ }
  return []
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentPath = ref<string | null>(null)
  const recentWorkspaces = ref<string[]>(readRecentWorkspaces())
  const favoriteWorkspaces = ref<string[]>(readFavoriteWorkspaces(recentWorkspaces.value))
  const listSettings = readWorkspaceListSettings()
  const workspaceSortMode = ref<WorkspaceSortMode>(listSettings.sortMode)
  const workspaceListCollapsed = ref(listSettings.collapsed)
  const workspaceNotes = ref(readRecord(WORKSPACE_NOTES_KEY, (value): value is string => typeof value === 'string' && value.length <= 200))
  const workspaceSummaries = ref(readRecord(WORKSPACE_SUMMARIES_KEY, isSummary))
  const workspaceSummaryErrors = ref<Record<string, string>>({})
  const summaryRequests = new Map<string, number>()
  let nextSummaryRequest = 0
  const svnInfo = ref<SvnInfo | null>(null)
  const statusList = ref<SvnStatus[]>([])
  const isLoading = ref(false)
  const statusIsStale = ref(false)
  const error = ref<string | null>(null)
  const gitignorePatterns = ref<GitignorePattern[]>([])
  const gitignoreMtime = ref<number | null>(null)
  const gitignoreWorkspacePath = ref<string | null>(null)

  const sortedWorkspaces = computed(() => {
    const paths = [...recentWorkspaces.value]
    if (workspaceSortMode.value === 'name') {
      const name = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() || path
      return paths.sort((a, b) => name(a).localeCompare(name(b), undefined, { numeric: true, sensitivity: 'base' }))
    }
    if (workspaceSortMode.value === 'modified') {
      return paths.sort((a, b) => (workspaceSummaries.value[b]?.latestModified ?? -1) - (workspaceSummaries.value[a]?.latestModified ?? -1))
    }
    return paths
  })

  const favoriteIdentities = computed(() => new Set(favoriteWorkspaces.value.map(workspaceIdentity)))
  const sortedFavoriteWorkspaces = computed(() => sortedWorkspaces.value.filter(isWorkspaceFavorite))

  function isWorkspaceFavorite(path: string): boolean {
    return favoriteIdentities.value.has(workspaceIdentity(path))
  }

  function setWorkspaceFavorite(path: string, favorite: boolean) {
    const savedPath = resolveWorkspacePath(path)
    if (!recentWorkspaces.value.includes(savedPath)) throw new Error('工作区已移除，无法修改收藏')
    if (isWorkspaceFavorite(savedPath) === favorite) return
    const next = favorite
      ? [...favoriteWorkspaces.value, savedPath]
      : favoriteWorkspaces.value.filter(item => workspaceIdentity(item) !== workspaceIdentity(savedPath))
    writeStoredValues([[FAVORITE_WORKSPACES_KEY, JSON.stringify(next)]], '收藏保存失败，请检查本地存储空间后重试')
    favoriteWorkspaces.value = next
  }

  const hasChanges = computed(() => {
    return statusList.value.some(s => s.status_code !== 'normal' && s.status_code !== '')
  })

  const modifiedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'modified').length
  })

  const changedCount = computed(() => {
    const changedStatuses = new Set(['modified', 'added', 'deleted', 'replaced'])
    return statusList.value.filter(s => changedStatuses.has(s.status_code) || s.prop_status === 'modified').length
  })

  const addedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'added').length
  })

  const deletedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'deleted').length
  })

  const unversionedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'unversioned').length
  })

  const conflictedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'conflicted' || s.prop_status === 'conflicted').length
  })

  const missingCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'missing').length
  })

  function resolveWorkspacePath(path: string): string {
    const key = workspaceIdentity(path)
    return recentWorkspaces.value.find(savedPath => workspaceIdentity(savedPath) === key) || path
  }

  function registerWorkspaces(paths: string[]): string[] {
    const next = [...recentWorkspaces.value]
    const existing = new Map(next.map(path => [workspaceIdentity(path), path]))
    const selected = new Set<string>()
    for (const path of paths) {
      if (typeof path !== 'string' || !path.trim()) continue
      const key = workspaceIdentity(path)
      let savedPath = existing.get(key)
      if (!savedPath) {
        savedPath = path
        existing.set(key, path)
        next.push(path)
      }
      selected.add(savedPath)
    }
    if (next.length !== recentWorkspaces.value.length) {
      writeStoredValues([[RECENT_WORKSPACES_KEY, JSON.stringify(next)]], '工作区列表保存失败，请检查本地存储空间后重试')
      recentWorkspaces.value = next
    }
    return [...selected]
  }

  function rememberWorkspace(path: string) {
    registerWorkspaces([path])
    writeStoredValues([[LAST_WORKSPACE_KEY, path]], '最近工作区保存失败，请检查本地存储空间后重试')
  }

  function setWorkspaceSortMode(mode: WorkspaceSortMode) {
    if (!['manual', 'name', 'modified'].includes(mode)) return
    writeStoredValues([[WORKSPACE_LIST_SETTINGS_KEY, JSON.stringify({ sortMode: mode, collapsed: workspaceListCollapsed.value })]], '排序方式保存失败，请检查本地存储空间后重试')
    workspaceSortMode.value = mode
  }

  function setWorkspaceListCollapsed(collapsed: boolean) {
    writeStoredValues([[WORKSPACE_LIST_SETTINGS_KEY, JSON.stringify({ sortMode: workspaceSortMode.value, collapsed })]], '折叠状态保存失败，请检查本地存储空间后重试')
    workspaceListCollapsed.value = collapsed
  }

  function reorderWorkspaces(sourcePath: string, targetPath: string, after: boolean) {
    if (sourcePath === targetPath || !recentWorkspaces.value.includes(sourcePath) || !recentWorkspaces.value.includes(targetPath)) return
    // 拖动以当前可见顺序为基础，保存后自动切到手动排序。
    const next = sortedWorkspaces.value.filter(path => path !== sourcePath)
    next.splice(next.indexOf(targetPath) + (after ? 1 : 0), 0, sourcePath)
    writeStoredValues([
      [RECENT_WORKSPACES_KEY, JSON.stringify(next)],
      [WORKSPACE_LIST_SETTINGS_KEY, JSON.stringify({ sortMode: 'manual', collapsed: workspaceListCollapsed.value })],
    ], '拖动顺序保存失败，请检查本地存储空间后重试')
    recentWorkspaces.value = next
    workspaceSortMode.value = 'manual'
  }

  function setWorkspaceNote(path: string, note: string) {
    if (!recentWorkspaces.value.includes(path)) throw new Error('工作区已移除，无法保存备注')
    if (note.length > 200) throw new Error('备注不能超过 200 个字符')
    const notes = { ...workspaceNotes.value }
    if (note.trim()) notes[path] = note.trim()
    else delete notes[path]
    try {
      localStorage.setItem(WORKSPACE_NOTES_KEY, JSON.stringify(notes))
    } catch { throw new Error('备注保存失败，请检查本地存储空间后重试') }
    workspaceNotes.value = notes
  }

  function beginWorkspaceSummary(path: string): number {
    const request = ++nextSummaryRequest
    summaryRequests.set(path, request)
    return request
  }

  function isWorkspaceSummaryCurrent(path: string, request: number): boolean {
    return recentWorkspaces.value.includes(path) && summaryRequests.get(path) === request
  }

  function persistWorkspaceSummaries() {
    try {
      localStorage.setItem(WORKSPACE_SUMMARIES_KEY, JSON.stringify(workspaceSummaries.value))
    } catch { /* 摘要缓存写入失败不影响本次扫描结果。 */ }
  }

  function setWorkspaceSummary(path: string, list: SvnStatus[], latestModified: number | null, request: number) {
    if (!isWorkspaceSummaryCurrent(path, request)) return
    const changedStatuses = new Set(['modified', 'added', 'deleted', 'replaced'])
    workspaceSummaries.value[path] = {
      changed: list.filter(file => changedStatuses.has(file.status_code) || file.prop_status === 'modified').length,
      missing: list.filter(file => file.status_code === 'missing').length,
      latestModified,
      scannedAt: Date.now(),
    }
    delete workspaceSummaryErrors.value[path]
    persistWorkspaceSummaries()
  }

  function setWorkspaceSummaryError(path: string, error: string, request: number) {
    if (!isWorkspaceSummaryCurrent(path, request)) return
    workspaceSummaryErrors.value[path] = error
    const previous = workspaceSummaries.value[path]
    // 读取失败保留上次统计，没有成功记录时保持未知，不能写成零。
    if (previous) {
      workspaceSummaries.value[path] = { ...previous, error }
      persistWorkspaceSummaries()
    }
  }

  function setCurrentPath(path: string, remember = true) {
    currentPath.value = path
    if (remember) rememberWorkspace(path)
  }

  function getLastWorkspacePath() {
    return localStorage.getItem(LAST_WORKSPACE_KEY)
  }

  function setStatusList(list: SvnStatus[]) {
    statusList.value = list
  }

  function setSvnInfo(info: SvnInfo | null) {
    svnInfo.value = info
  }

  function setStatusIsStale(value: boolean) {
    statusIsStale.value = value
  }

  function setLoading(value: boolean) {
    isLoading.value = value
  }

  function setError(value: string | null) {
    error.value = value
  }

  function setGitignorePatterns(patterns: GitignorePattern[]) {
    gitignorePatterns.value = patterns
  }

  function setGitignoreMtime(mtime: number | null) {
    gitignoreMtime.value = mtime
  }

  function setGitignoreWorkspacePath(path: string | null) {
    gitignoreWorkspacePath.value = path
  }

  function clearWorkspace(forgetLastWorkspace = true) {
    if (forgetLastWorkspace) localStorage.removeItem(LAST_WORKSPACE_KEY)
    resetWorkspaceState()
  }

  // 加载失败时仅清空内存状态，保留下次启动需要恢复的工作区。
  function resetWorkspaceState() {
    currentPath.value = null
    svnInfo.value = null
    statusList.value = []
    isLoading.value = false
    statusIsStale.value = false
    error.value = null
    gitignorePatterns.value = []
    gitignoreMtime.value = null
    gitignoreWorkspacePath.value = null
  }

  function removeRecentWorkspace(path: string) {
    path = resolveWorkspacePath(path)
    if (!recentWorkspaces.value.includes(path)) return
    const nextWorkspaces = recentWorkspaces.value.filter(item => item !== path)
    const nextFavorites = favoriteWorkspaces.value.filter(item => workspaceIdentity(item) !== workspaceIdentity(path))
    const nextNotes = { ...workspaceNotes.value }
    const nextSummaries = { ...workspaceSummaries.value }
    delete nextNotes[path]
    delete nextSummaries[path]
    const entries: [string, string | null][] = [
      [RECENT_WORKSPACES_KEY, JSON.stringify(nextWorkspaces)],
      [FAVORITE_WORKSPACES_KEY, JSON.stringify(nextFavorites)],
      [WORKSPACE_NOTES_KEY, JSON.stringify(nextNotes)],
      [WORKSPACE_SUMMARIES_KEY, JSON.stringify(nextSummaries)],
    ]
    if (currentPath.value === path) entries.push([LAST_WORKSPACE_KEY, null])
    writeStoredValues(entries, '工作区移除失败，请检查本地存储空间后重试')
    invalidateWorkspaceSnapshot(path)
    summaryRequests.delete(path)
    delete workspaceSummaryErrors.value[path]
    recentWorkspaces.value = nextWorkspaces
    favoriteWorkspaces.value = nextFavorites
    workspaceNotes.value = nextNotes
    workspaceSummaries.value = nextSummaries
    if (currentPath.value === path) clearWorkspace(false)
  }

  return {
    currentPath,
    recentWorkspaces,
    sortedWorkspaces,
    favoriteWorkspaces,
    sortedFavoriteWorkspaces,
    isWorkspaceFavorite,
    setWorkspaceFavorite,
    workspaceSortMode,
    workspaceListCollapsed,
    resolveWorkspacePath,
    registerWorkspaces,
    setWorkspaceSortMode,
    setWorkspaceListCollapsed,
    reorderWorkspaces,
    workspaceNotes,
    workspaceSummaries,
    workspaceSummaryErrors,
    setWorkspaceNote,
    beginWorkspaceSummary,
    isWorkspaceSummaryCurrent,
    setWorkspaceSummary,
    setWorkspaceSummaryError,
    svnInfo,
    statusList,
    isLoading,
    statusIsStale,
    setStatusIsStale,
    error,
    hasChanges,
    modifiedCount,
    changedCount,
    addedCount,
    deletedCount,
    unversionedCount,
    conflictedCount,
    missingCount,
    gitignorePatterns,
    gitignoreMtime,
    gitignoreWorkspacePath,
    setCurrentPath,
    rememberWorkspace,
    removeRecentWorkspace,
    getLastWorkspacePath,
    setStatusList,
    setSvnInfo,
    setLoading,
    setError,
    setGitignorePatterns,
    setGitignoreMtime,
    setGitignoreWorkspacePath,
    clearWorkspace,
    resetWorkspaceState,
  }
})
