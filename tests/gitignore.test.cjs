const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')

const compiled = ts.transpileModule(fs.readFileSync('src/utils/gitignore.ts', 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS },
}).outputText
const context = { exports: {} }
vm.runInNewContext(compiled, context)
const { filterUnversionedByGitignore, parseGitignore } = context.exports

test('gitignore hides unversioned paths but keeps tracked changes', () => {
  const patterns = parseGitignore('*.log')
  const statuses = [
    { path: 'existing.log', status_code: 'modified' },
    { path: 'new.log', status_code: 'unversioned' },
    { path: 'source.ts', status_code: 'modified' },
  ]
  const visible = filterUnversionedByGitignore(statuses, patterns)
  assert.deepEqual(Array.from(visible, status => status.path), ['existing.log', 'source.ts'])
})
