const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')

const compiled = ts.transpileModule(fs.readFileSync('src/utils/version.ts', 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS },
}).outputText
const context = { exports: {} }
vm.runInNewContext(compiled, context)
const { isNewerVersion } = context.exports

test('detects newer stable releases numerically', () => {
  assert.equal(isNewerVersion('v0.6.5', '0.6.4'), true)
  assert.equal(isNewerVersion('v0.10.0', '0.9.9'), true)
  assert.equal(isNewerVersion('1.0.0', '0.99.99'), true)
})

test('ignores equal, older, prerelease and malformed tags', () => {
  assert.equal(isNewerVersion('v0.6.4', '0.6.4'), false)
  assert.equal(isNewerVersion('v0.6.3', '0.6.4'), false)
  assert.equal(isNewerVersion('v0.7.0-beta.1', '0.6.4'), false)
  assert.equal(isNewerVersion('nightly', '0.6.4'), false)
})

test('prerelease builds are offered the matching or a newer stable release', () => {
  assert.equal(isNewerVersion('v0.6.5', '0.6.5-rc.1'), true)
  assert.equal(isNewerVersion('v0.6.6', '0.6.5-rc.1'), true)
  assert.equal(isNewerVersion('v0.6.4', '0.6.5-rc.1'), false)
  assert.equal(isNewerVersion('v0.6.5-rc.2', '0.6.5-rc.1'), false)
})
