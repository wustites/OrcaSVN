import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { StashEntry } from '@/types'

const STASHES_KEY = 'orcasvn-stashes'

function readEntries(): StashEntry[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STASHES_KEY) || '[]')
    if (!Array.isArray(value)) return []
    return value.filter((entry): entry is StashEntry => {
      if (!entry || typeof entry !== 'object') return false
      const candidate = entry as Partial<StashEntry>
      return typeof candidate.id === 'string'
        && typeof candidate.name === 'string'
        && typeof candidate.workspacePath === 'string'
        && typeof candidate.createdAt === 'string'
        && typeof candidate.patch === 'string'
        && Array.isArray(candidate.files)
        && typeof candidate.hunkCount === 'number'
    })
  } catch {
    return []
  }
}

export const useStashStore = defineStore('stash', () => {
  const entries = ref<StashEntry[]>(readEntries())

  const persist = (nextEntries: StashEntry[]) => {
    localStorage.setItem(STASHES_KEY, JSON.stringify(nextEntries))
  }

  const currentEntries = computed(() => (workspacePath: string | null) => {
    if (!workspacePath) return []
    return entries.value.filter(entry => entry.workspacePath === workspacePath)
  })

  const addEntry = (entry: StashEntry) => {
    const nextEntries = [entry, ...entries.value]
    // Persist before publishing the new state. A quota or storage failure must
    // not make the UI claim that the only copy of a patch was saved.
    persist(nextEntries)
    entries.value = nextEntries
  }

  const removeEntry = (id: string) => {
    const nextEntries = entries.value.filter(entry => entry.id !== id)
    persist(nextEntries)
    entries.value = nextEntries
  }

  return { entries, currentEntries, addEntry, removeEntry }
})
