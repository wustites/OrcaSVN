import type { SvnInfo } from '@/types'

const SVN_INFO_CACHE_KEY = 'orcasvn-svn-info-cache-v1'
export const SVN_INFO_CACHE_TTL_MS = 24 * 60 * 60 * 1000

export type SvnInfoMetadata = Omit<SvnInfo, 'revision'>

interface CachedSvnInfoMetadata {
  cachedAt: number
  metadata: SvnInfoMetadata
}

type SvnInfoCache = Record<string, CachedSvnInfoMetadata>

function readCache(): SvnInfoCache {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(SVN_INFO_CACHE_KEY) || '{}')
    return value && typeof value === 'object' ? value as SvnInfoCache : {}
  } catch {
    return {}
  }
}

function isMetadata(value: unknown): value is SvnInfoMetadata {
  if (!value || typeof value !== 'object') return false
  const metadata = value as Partial<SvnInfoMetadata>
  return [metadata.path, metadata.url, metadata.repository_root, metadata.node_kind, metadata.schedule]
    .every(field => typeof field === 'string')
}

export function getCachedSvnInfoMetadata(path: string): SvnInfoMetadata | null {
  const cached = readCache()[path]
  if (!cached || typeof cached.cachedAt !== 'number' || !isMetadata(cached.metadata)) return null
  if (Date.now() - cached.cachedAt > SVN_INFO_CACHE_TTL_MS) return null
  return cached.metadata
}

export function cacheSvnInfoMetadata(path: string, metadata: SvnInfoMetadata): void {
  try {
    const cache = readCache()
    cache[path] = { cachedAt: Date.now(), metadata }
    localStorage.setItem(SVN_INFO_CACHE_KEY, JSON.stringify(cache))
  } catch {
    // Caching is an optimization; a storage failure must not block SVN operations.
  }
}
