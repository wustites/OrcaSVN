import type { SvnStatus } from '@/types'

export type ChangelistRow =
  | { kind: 'group'; key: string; name: string | null; count: number }
  | { kind: 'file'; key: string; file: SvnStatus; fileIndex: number }

export interface GroupedChangelists {
  rows: ChangelistRow[]
  files: { file: SvnStatus; rowIndex: number }[]
}

/** 仅从筛选后的文件建立非空分组，未分组文件排在最后。 */
export function groupChangelistFiles(files: SvnStatus[]): GroupedChangelists {
  const groups = new Map<string | null, SvnStatus[]>()
  for (const file of files) {
    const name = file.changelist || null
    const entries = groups.get(name) || []
    entries.push(file)
    groups.set(name, entries)
  }
  const entries = [...groups.entries()].filter(([name]) => name !== null)
  const ungrouped = groups.get(null)
  if (ungrouped) entries.push([null, ungrouped])

  const result: GroupedChangelists = { rows: [], files: [] }
  for (const [name, group] of entries) {
    result.rows.push({ kind: 'group', key: JSON.stringify(['group', name]), name, count: group.length })
    for (const file of group) {
      // 文件序号和虚拟行序号分开，方向键不会选中分组标题。
      const fileIndex = result.files.length
      result.files.push({ file, rowIndex: result.rows.length })
      result.rows.push({ kind: 'file', key: JSON.stringify(['file', file.path]), file, fileIndex })
    }
  }
  return result
}
