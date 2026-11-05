const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')
const vue = require('vue')

function harness() {
  const toasts = []
  const store = vue.reactive({ error: null, setError(value) { store.error = value } })
  const mocks = {
    vue,
    'vue-i18n': { useI18n: () => ({ t: key => key }) },
    'element-plus/es/components/message/index': { ElMessage: { error(options) {
      const toast = { ...options, open: true, close() {
        if (!toast.open) return
        toast.open = false
        // Element Plus runs onClose asynchronously, after the leave transition starts.
        Promise.resolve().then(() => options.onClose())
      } }
      toasts.push(toast)
      return toast
    } } },
    '@/stores/workspace': { useWorkspaceStore: () => store },
  }
  const context = { exports: {}, require: name => { assert.ok(name in mocks, name); return mocks[name] } }
  const source = fs.readFileSync('src/composables/useWorkspaceErrorToast.ts', 'utf8')
  vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, context)
  context.exports.useWorkspaceErrorToast()
  return { store, toasts }
}
const settle = async () => { for (let i = 0; i < 5; i++) await vue.nextTick() }

test('an error opens one persistent, labeled toast until the store clears it', async () => {
  const { store, toasts } = harness()
  store.setError('boom')
  await settle()
  assert.equal(toasts.length, 1)
  assert.equal(toasts[0].duration, 0)
  assert.equal(toasts[0].message, 'common.error：boom')
  store.setError(null)
  await settle()
  assert.equal(toasts[0].open, false)
})

test('the same error is shown again after the user dismissed it', async () => {
  const { store, toasts } = harness()
  store.setError('boom')
  await settle()
  toasts[0].close()
  await settle()
  assert.equal(store.error, null)
  store.setError('boom')
  await settle()
  assert.equal(toasts.length, 2)
  assert.equal(toasts[1].open, true)
})

test('replacing a toast does not let its late close clear the new error', async () => {
  const { store, toasts } = harness()
  store.setError('first')
  await settle()
  store.setError('second')
  await settle()
  assert.equal(store.error, 'second')
  assert.equal(toasts[0].open, false)
  assert.equal(toasts[1].open, true)
})
