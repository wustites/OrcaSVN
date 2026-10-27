export interface DiffIndex {
  offsets: Uint32Array
  /** Line number in the original file for each row; 0 when the row has none. */
  oldLines: Uint32Array
  /** Line number in the new file for each row; 0 when the row has none. */
  newLines: Uint32Array
  added: number
  removed: number
  maxColumns: number
}
export interface DiffRow {
  index: number
  oldLine: number
  newLine: number
  marker: string
  text: string
  className: string
  start: number
}

const HUNK_HEADER = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/

function classify(line: string) {
  if (line.startsWith('+') && !line.startsWith('+++')) return 'added'
  if (line.startsWith('-') && !line.startsWith('---')) return 'removed'
  if (line.startsWith('@@') || line.startsWith('+++') || line.startsWith('---')) return 'meta'
  return 'context'
}

export function indexDiff(text: string): DiffIndex {
  const offsets: number[] = []
  const oldLines: number[] = []
  const newLines: number[] = []
  let oldNext = 0, newNext = 0, oldRemain = 0, newRemain = 0
  let added = 0, removed = 0, maxColumns = 0
  if (text) {
    let start = 0
    do {
      offsets.push(start)
      const newline = text.indexOf('\n', start)
      const end = newline === -1 ? text.length : newline
      const line = text.slice(start, end).replace(/\r$/, '')
      const type = classify(line)
      let oldLine = 0, newLine = 0
      const hunk = oldRemain > 0 || newRemain > 0 ? null : HUNK_HEADER.exec(line)
      if (hunk) {
        oldNext = Number(hunk[1]); oldRemain = hunk[2] === undefined ? 1 : Number(hunk[2])
        newNext = Number(hunk[3]); newRemain = hunk[4] === undefined ? 1 : Number(hunk[4])
      } else if (oldRemain > 0 || newRemain > 0) {
        // Inside a hunk the first character decides the side, so content such as "-- x" is counted correctly.
        const side = line[0]
        if (side === '-') { oldLine = oldNext++; oldRemain-- }
        else if (side === '+') { newLine = newNext++; newRemain-- }
        else if (side !== '\\') { oldLine = oldNext++; newLine = newNext++; oldRemain--; newRemain-- }
      }
      oldLines.push(oldLine)
      newLines.push(newLine)
      if (type === 'added') added++
      if (type === 'removed') removed++
      let columns = 0
      for (let i = type === 'added' || type === 'removed' ? 1 : 0; i < line.length; i++) {
        const code = line.charCodeAt(i)
        // Reserve enough width for tabs and wide glyphs, even on offscreen lines.
        columns += code === 9 ? 8 - columns % 8 : code > 255 ? 2 : 1
      }
      maxColumns = Math.max(maxColumns, columns)
      if (newline === -1) break
      start = newline + 1
    } while (start <= text.length)
  }
  return { offsets: Uint32Array.from(offsets), oldLines: Uint32Array.from(oldLines), newLines: Uint32Array.from(newLines), added, removed, maxColumns }
}

export function getDiffRows(text: string, index: DiffIndex, start: number, end: number): DiffRow[] {
  const rows: DiffRow[] = []
  for (let row = start; row < Math.min(end, index.offsets.length); row++) {
    const offset = index.offsets[row]
    const limit = row + 1 < index.offsets.length ? index.offsets[row + 1] - 1 : text.length
    const line = text.slice(offset, limit).replace(/\r$/, '')
    const type = classify(line)
    const changed = type === 'added' || type === 'removed'
    rows.push({ index: row + 1, oldLine: index.oldLines[row], newLine: index.newLines[row], marker: changed ? line[0] : type === 'meta' ? '@' : '',
      text: changed ? line.slice(1) : line, className: `diff-${type}`, start: offset + Number(changed) })
  }
  return rows
}

export function lineAtOffset(index: DiffIndex, offset: number): number {
  let left = 0, right = index.offsets.length
  while (left < right) {
    const middle = (left + right) >>> 1
    if (index.offsets[middle] <= offset) left = middle + 1
    else right = middle
  }
  return Math.max(0, left - 1)
}

export function findDiffMatch(text: string, query: string, from: number, direction: 'next' | 'previous'): number {
  if (!query) return -1
  if (direction === 'next') {
    const match = text.indexOf(query, Math.max(0, from))
    return match === -1 ? text.indexOf(query) : match
  }
  const match = from < 0 ? -1 : text.lastIndexOf(query, from)
  return match === -1 ? text.lastIndexOf(query) : match
}
