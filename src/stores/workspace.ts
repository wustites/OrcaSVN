import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { SvnStatus, SvnInfo } from '@/types'
import type { GitignorePattern } from '@/utils/gitignore'

const LAST_WORKSPACE_KEY = 'orcasvn-last-workspace'

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentPath = ref<string | null>(null)
  const svnInfo = ref<SvnInfo | null>(null)
  const statusList = ref<SvnStatus[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const gitignorePatterns = ref<GitignorePattern[]>([])
  const gitignoreMtime = ref<number | null>(null)

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

  function setCurrentPath(path: string) {
    currentPath.value = path
    localStorage.setItem(LAST_WORKSPACE_KEY, path)
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

  function clearWorkspace() {
    localStorage.removeItem(LAST_WORKSPACE_KEY)
    currentPath.value = null
    svnInfo.value = null
    statusList.value = []
    error.value = null
    gitignorePatterns.value = []
    gitignoreMtime.value = null
  }

  return {
    currentPath,
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
    setCurrentPath,
    getLastWorkspacePath,
    setStatusList,
    setSvnInfo,
    setLoading,
    setError,
    setGitignorePatterns,
    setGitignoreMtime,
    clearWorkspace,
  }
})
