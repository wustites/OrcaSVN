import type { StashFile, StashHunk } from '@/types'

export function parseUnifiedDiff(path: string, diff: string): StashFile | null {
  const lines = diff.replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n')
  if (lines[lines.length - 1] === '') lines.pop()

  const firstHunk = lines.findIndex(line => line.startsWith('@@'))
  if (firstHunk < 0) return null

  const header = lines.slice(0, firstHunk)
  const hunks: StashHunk[] = []
  let index = firstHunk

  while (index < lines.length) {
    if (!lines[index].startsWith('@@')) {
      index += 1
      continue
    }

    const headerLine = lines[index]
    const hunkLines: string[] = []
    index += 1
    while (index < lines.length && !lines[index].startsWith('@@')) {
      hunkLines.push(lines[index])
      index += 1
    }

    hunks.push({
      id: `${path}::${hunks.length}`,
      header: headerLine,
      lines: hunkLines,
      added: hunkLines.filter(line => line.startsWith('+')).length,
      removed: hunkLines.filter(line => line.startsWith('-')).length,
    })
  }

  return hunks.length > 0 ? { path, header, hunks } : null
}

export function buildPatch(files: StashFile[], selectedHunks: Set<string>): string {
  const patches: string[] = []

  for (const file of files) {
    const selected = file.hunks.filter(hunk => selectedHunks.has(hunk.id))
    if (selected.length === 0) continue

    patches.push([
      ...file.header,
      ...selected.flatMap(hunk => [hunk.header, ...hunk.lines]),
    ].join('\n'))
  }

  return patches.length > 0 ? `${patches.join('\n')}\n` : ''
}

export function createStashId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID()
  return `stash-${Date.now()}-${Math.random().toString(36).slice(2)}`
}
