export interface GitignorePattern {
  negation: boolean
  regex: RegExp
  dirOnly: boolean
}

export function parseGitignore(content: string): GitignorePattern[] {
  const patterns: GitignorePattern[] = []

  for (const line of content.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) continue

    let pattern = trimmed
    let negation = false
    if (pattern.startsWith('!')) {
      negation = true
      pattern = pattern.slice(1).trimStart()
    }
    if (!pattern) continue

    const dirOnly = pattern.endsWith('/')
    if (dirOnly) pattern = pattern.slice(0, -1)

    const anchored = pattern.startsWith('/')
    if (anchored) pattern = pattern.slice(1)

    const regex = gitignorePatternToRegex(pattern, anchored)
    patterns.push({ negation, regex, dirOnly })
  }

  return patterns
}

function gitignorePatternToRegex(pattern: string, anchored: boolean): RegExp {
  let regexStr = ''

  for (let i = 0; i < pattern.length; i++) {
    const ch = pattern[i]
    if (ch === '*') {
      if (i + 1 < pattern.length && pattern[i + 1] === '*') {
        regexStr += '.*'
        i++
        if (i + 1 < pattern.length && pattern[i + 1] === '/') i++
      } else {
        regexStr += '[^/]*'
      }
    } else if (ch === '?') {
      regexStr += '[^/]'
    } else if ('.+^${}()|[]\\'.includes(ch)) {
      regexStr += '\\' + ch
    } else {
      regexStr += ch
    }
  }

  if (!anchored && !pattern.includes('/')) {
    return new RegExp(`(^|/)${regexStr}$`)
  }
  return new RegExp(`^${regexStr}$`)
}

function isDirPath(path: string): boolean {
  return path.endsWith('/')
}

export function isIgnored(filePath: string, patterns: GitignorePattern[]): boolean {
  let ignored = false
  const normalPath = filePath.replace(/\\/g, '/')
  for (const p of patterns) {
    if (p.dirOnly && !isDirPath(normalPath) && !isDirPath(filePath)) continue
    if (p.regex.test(normalPath)) {
      ignored = !p.negation
    }
  }
  return ignored
}
