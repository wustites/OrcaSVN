const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')

const compiled = ts.transpileModule(fs.readFileSync('src/utils/stash.ts', 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS },
}).outputText
const context = { exports: {}, crypto: { randomUUID: () => 'test-id' } }
vm.runInNewContext(compiled, context)
const { buildPatch, createStashId, parseUnifiedDiff } = context.exports

const diff = [
  'Index: src/example.txt',
  '===================================================================',
  '--- src/example.txt\t(revision 1)',
  '+++ src/example.txt\t(working copy)',
  '@@ -1,2 +1,2 @@',
  '-first',
  '+changed first',
  ' second',
  '@@ -10,2 +10,2 @@',
  '-tenth',
  '+changed tenth',
  ' eleventh',
  '',
].join('\n')

test('parses independent hunks and stable selection IDs', () => {
  const file = parseUnifiedDiff('src/example.txt', diff)
  assert.ok(file)
  assert.equal(file.hunks.length, 2)
  assert.equal(file.hunks.map(hunk => hunk.id).join(','), 'src/example.txt::0,src/example.txt::1')
})

test('builds a patch containing only selected hunks and the required file header', () => {
  const file = parseUnifiedDiff('src/example.txt', diff)
  const patch = buildPatch([file], new Set(['src/example.txt::1']))
  assert.match(patch, /Index: src\/example\.txt/)
  assert.doesNotMatch(patch, /changed first/)
  assert.match(patch, /changed tenth/)
  assert.ok(patch.endsWith('\n'))
})

test('ignores non-text diffs and returns an empty patch for an empty selection', () => {
  assert.equal(parseUnifiedDiff('image.png', 'Cannot display: file marked as a binary type.'), null)
  assert.equal(buildPatch([], new Set()), '')
  assert.equal(createStashId(), 'test-id')
})
