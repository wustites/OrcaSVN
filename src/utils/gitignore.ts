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
        while (i + 1 < pattern.length && pattern[i + 1] === '*') i++
        if (i + 1 < pattern.length && pattern[i + 1] === '/') {
          regexStr += '(?:.*/)?'
          i++
        } else {
          regexStr += '.*'
        }
      } else {
        regexStr += '[^/]*'
      }
    } else if (ch === '?') {
      regexStr += '[^/]'
    } else if (ch === '[') {
      const closingBracket = pattern.indexOf(']', i + 1)
      if (closingBracket === -1) {
        regexStr += '\\['
        continue
      }

      let characterClass = pattern.slice(i + 1, closingBracket)
      if (characterClass.startsWith('!')) {
        characterClass = `^/${characterClass.slice(1)}`
      } else if (characterClass.startsWith('^')) {
        characterClass = `\\${characterClass}`
      }
      regexStr += `[${characterClass.replace(/\\/g, '\\\\')}]`
      i = closingBracket
    } else if (ch === '\\' && i + 1 < pattern.length) {
      i++
      const escaped = pattern[i]
      regexStr += '.+^${}()|[]\\'.includes(escaped) ? `\\${escaped}` : escaped
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

export function isIgnored(filePath: string, patterns: GitignorePattern[]): boolean {
  let ignored = false
  const normalPath = filePath.replace(/\\/g, '/').replace(/^\.\//, '').replace(/\/+$/, '')
  const pathCandidates = [normalPath]
  let separator = normalPath.lastIndexOf('/')
  while (separator >= 0) {
    pathCandidates.push(normalPath.slice(0, separator))
    separator = normalPath.lastIndexOf('/', separator - 1)
  }

  for (const p of patterns) {
    if (pathCandidates.some(candidate => p.regex.test(candidate))) {
      ignored = !p.negation
    }
  }
  return ignored
}
