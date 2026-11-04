export function retainedSelection(selected: string | null, paths: readonly string[]): string | null {
  if (selected === null) return null
  return paths.includes(selected) ? selected : null
}
