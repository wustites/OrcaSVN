import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { SvnStatus, SvnInfo } from '@/types'
import type { GitignorePattern } from '@/utils/gitignore'

const LAST_WORKSPACE_KEY = 'orcasvn-last-workspace'
const RECENT_WORKSPACES_KEY = 'orcasvn-recent-workspaces'
const MAX_RECENT_WORKSPACES = 10

function readRecentWorkspaces(): string[] {
  try {
    const stored = JSON.parse(localStorage.getItem(RECENT_WORKSPACES_KEY) || '[]')
    if (Array.isArray(stored)) {
      return stored.filter((path): path is string => typeof path === 'string' && path.length > 0)
    }
  } catch {
    // Ignore malformed data from older/manual installations.
  }
  return []
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentPath = ref<string | null>(null)
  const recentWorkspaces = ref<string[]>(readRecentWorkspaces())
  const svnInfo = ref<SvnInfo | null>(null)
  const statusList = ref<SvnStatus[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const gitignorePatterns = ref<GitignorePattern[]>([])
  const gitignoreMtime = ref<number | null>(null)
  const gitignoreWorkspacePath = ref<string | null>(null)

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

  function rememberWorkspace(path: string) {
    recentWorkspaces.value = [path, ...recentWorkspaces.value.filter(item => item !== path)]
      .slice(0, MAX_RECENT_WORKSPACES)
    localStorage.setItem(RECENT_WORKSPACES_KEY, JSON.stringify(recentWorkspaces.value))
    localStorage.setItem(LAST_WORKSPACE_KEY, path)
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

  function clearWorkspace() {
    localStorage.removeItem(LAST_WORKSPACE_KEY)
    currentPath.value = null
    svnInfo.value = null
    statusList.value = []
    error.value = null
    gitignorePatterns.value = []
    gitignoreMtime.value = null
    gitignoreWorkspacePath.value = null
  }

  function removeRecentWorkspace(path: string) {
    recentWorkspaces.value = recentWorkspaces.value.filter(item => item !== path)
    localStorage.setItem(RECENT_WORKSPACES_KEY, JSON.stringify(recentWorkspaces.value))
    if (currentPath.value === path) clearWorkspace()
  }

  return {
    currentPath,
    recentWorkspaces,
    svnInfo,
    statusList,
    isLoading,
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
  }
})
