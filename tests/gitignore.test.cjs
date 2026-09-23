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
const { filterByGitignore, parseGitignore } = context.exports

test('gitignore hides matching tracked and unversioned paths', () => {
  const patterns = parseGitignore('*.log\n!keep.log')
  const statuses = [
    { path: 'existing.log', status_code: 'modified' },
    { path: 'new.log', status_code: 'unversioned' },
    { path: 'keep.log', status_code: 'modified' },
    { path: 'source.ts', status_code: 'modified' },
  ]
  const visible = filterByGitignore(statuses, patterns)
  assert.deepEqual(Array.from(visible, status => status.path), ['keep.log', 'source.ts'])
  assert.equal(filterByGitignore(statuses, []).length, statuses.length)
})
