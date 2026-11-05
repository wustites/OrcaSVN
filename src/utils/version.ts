/** Parses `v1.2.3` / `1.2.3`; prerelease or malformed tags return null. */
export function parseStableVersion(tag: string): number[] | null {
  const match = /^v?(\d+)\.(\d+)\.(\d+)$/.exec(tag.trim())
  return match ? match.slice(1).map(Number) : null
}

/** Only a stable `candidate` can be newer; `current` may be a prerelease of its own version. */
export function isNewerVersion(candidate: string, current: string): boolean {
  const next = parseStableVersion(candidate)
  const [currentCore, currentPrerelease] = current.trim().split(/-(.*)/s)
  const now = parseStableVersion(currentCore)
  if (!next || !now) return false
  for (let i = 0; i < 3; i++) {
    if (next[i] !== now[i]) return next[i] > now[i]
  }
  return Boolean(currentPrerelease)
}
