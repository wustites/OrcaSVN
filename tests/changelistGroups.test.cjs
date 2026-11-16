const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const vm = require('node:vm')
const ts = require('typescript')

const context = { exports: {} }
const code = ts.transpileModule(fs.readFileSync('src/utils/changelistGroups.ts', 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
}).outputText
vm.runInNewContext(code, context)
const { groupChangelistFiles } = context.exports
const file = (path, changelist, status_code = 'modified') => ({ path, changelist, status_code })

test('只建立筛选后仍有文件的分组，保留未分组文件且空结果没有标题', () => {
  const all = [file('a', '临时代码'), file('b', '修复', 'missing'), file('c', null), file('d', '')]
  const result = groupChangelistFiles(all.filter(item => item.status_code === 'modified'))
  assert.deepEqual(Array.from(result.rows.filter(row => row.kind === 'group'), row => [row.name, row.count]), [
    ['临时代码', 1], [null, 2],
  ])
  assert.deepEqual(Array.from(result.files, entry => entry.file.path), ['a', 'c', 'd'])
  assert.equal(groupChangelistFiles([]).rows.length, 0)
})

test('同组文件聚合后文件序号与虚拟行位置对应，键盘导航跨组不会选中标题', () => {
  const result = groupChangelistFiles([file('a', '甲'), file('b', '乙'), file('c', '甲'), file('d')])
  assert.deepEqual(Array.from(result.files, entry => entry.file.path), ['a', 'c', 'b', 'd'])
  assert.deepEqual(Array.from(result.files, entry => entry.rowIndex), [1, 2, 4, 6])
  result.files.forEach((entry, index) => {
    const row = result.rows[entry.rowIndex]
    assert.equal(row.kind, 'file')
    assert.equal(row.fileIndex, index)
    assert.equal(row.file, entry.file)
  })
})

test('分组名称与文件路径或未分组名称相同仍有独立键', () => {
  const result = groupChangelistFiles([file('未分组', '未分组'), file('other', null), file('null', 'null')])
  assert.equal(new Set(result.rows.map(row => row.key)).size, result.rows.length)
  assert.deepEqual(Array.from(result.rows.filter(row => row.kind === 'group'), row => row.name), ['未分组', 'null', null])
})
