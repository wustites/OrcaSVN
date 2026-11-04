const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')
const context = { exports: {} }
vm.runInNewContext(ts.transpileModule(fs.readFileSync('src/utils/statusSelection.ts', 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, context)
const { retainedSelection } = context.exports

test('clears a diff selection that left the status list and keeps one that remains', () => {
  assert.equal(retainedSelection(null, []), null)
  assert.equal(retainedSelection('文档/说明.txt', ['文档/说明.txt', 'plain.txt']), '文档/说明.txt')
  assert.equal(retainedSelection('文档/说明.txt', ['plain.txt']), null)
  assert.equal(retainedSelection('文档/说明.txt', []), null)
})
