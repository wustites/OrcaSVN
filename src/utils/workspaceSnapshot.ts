import type { SvnInfo, SvnStatus } from '@/types'

const KEY = 'orcasvn-workspace-snapshots-v2'
const TTL_MS = 24 * 60 * 60 * 1000
const MAX_WORKSPACES = 5
const MAX_FILES = 1000
const MAX_BYTES = 1024 * 1024

export interface WorkspaceSnapshot {
  path: string
  savedAt: number
  gitignoreEnabled: boolean
  statusList: SvnStatus[]
  svnInfo: SvnInfo
}

function readSnapshots(): WorkspaceSnapshot[] {
  try {
    const text = localStorage.getItem(KEY)
    if (!text || text.length > MAX_BYTES) return []
    const entries: unknown = JSON.parse(text)
    if (!Array.isArray(entries)) return []
    return entries.filter((entry): entry is WorkspaceSnapshot => {
      if (!entry || typeof entry !== 'object' || typeof entry.path !== 'string'
        || typeof entry.savedAt !== 'number' || !Number.isFinite(entry.savedAt)
        || entry.savedAt > Date.now() || Date.now() - entry.savedAt > TTL_MS
        || typeof entry.gitignoreEnabled !== 'boolean'
        || !Array.isArray(entry.statusList) || entry.statusList.length > MAX_FILES) return false
      const info = entry.svnInfo
      return info && ['path', 'url', 'repository_root', 'node_kind', 'schedule'].every(key => typeof info[key] === 'string')
        && Number.isFinite(info.revision)
        && entry.statusList.every((file: SvnStatus) => file
          && ['path', 'status', 'status_code', 'prop_status'].every(key => typeof file[key as keyof SvnStatus] === 'string')
          && ['locked', 'history', 'switched'].every(key => typeof file[key as keyof SvnStatus] === 'boolean')
          && (file.changelist == null || typeof file.changelist === 'string'))
    }).slice(0, MAX_WORKSPACES)
  } catch { return [] }
}

export function getWorkspaceSnapshot(path: string, gitignoreEnabled: boolean): WorkspaceSnapshot | null {
  return readSnapshots().find(entry => entry.path === path && entry.gitignoreEnabled === gitignoreEnabled) || null
}

export function invalidateWorkspaceSnapshot(path: string): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(readSnapshots().filter(entry => entry.path !== path)))
  } catch { /* Storage is optional. */ }
}

export function cacheWorkspaceSnapshot(path: string, statusList: SvnStatus[], svnInfo: SvnInfo, gitignoreEnabled: boolean): void {
  try {
    const entries = readSnapshots().filter(entry => entry.path !== path)
    // Persist only small change lists, avoiding synchronous serialization of
    // very large results. Never leave an older snapshot behind when skipped.
    if (statusList.length <= MAX_FILES) {
      const snapshot = { path, statusList, svnInfo, gitignoreEnabled, savedAt: Date.now() }
      if (JSON.stringify(snapshot).length <= MAX_BYTES) entries.unshift(snapshot)
    }
    entries.splice(MAX_WORKSPACES)
    while (JSON.stringify(entries).length > MAX_BYTES) entries.pop()
    localStorage.setItem(KEY, JSON.stringify(entries))
  } catch { /* A storage failure must not fail a successful SVN scan. */ }
}
