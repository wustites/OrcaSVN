import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface SvnStatusItem {
  path: string
  status: string
  status_code: string
  prop_status: string
  locked: boolean
  history: boolean
  switched: boolean
}

export interface SvnLogEntry {
  revision: number
  author: string
  date: string
  message: string
}

export interface SvnInfo {
  path: string
  url: string
  repository_root: string
  revision: number
  node_kind: string
  schedule: string
}

export interface DiffResult {
  path: string
  diff: string
  old_revision: number
  new_revision: number
}

export interface BlameLine {
  revision: number
  author: string
  line: string
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const currentPath = ref<string | null>(null)
  const svnInfo = ref<SvnInfo | null>(null)
  const statusList = ref<SvnStatusItem[]>([])
  const isLoading = ref(false)
  const error = ref<string | null>(null)

  const hasChanges = computed(() => {
    return statusList.value.some(s => s.status_code !== ' ' && s.status_code !== '')
  })

  const modifiedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'M').length
  })

  const addedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'A').length
  })

  const deletedCount = computed(() => {
    return statusList.value.filter(s => s.status_code === 'D').length
  })

  function setCurrentPath(path: string) {
    currentPath.value = path
    console.log('[store] setCurrentPath:', path)
  }

  function clearWorkspace() {
    currentPath.value = null
    svnInfo.value = null
    statusList.value = []
    error.value = null
    console.log('[store] clearWorkspace')
  }

  return {
    currentPath,
    svnInfo,
    statusList,
    isLoading,
    error,
    hasChanges,
    modifiedCount,
    addedCount,
    deletedCount,
    setCurrentPath,
    clearWorkspace,
  }
})
