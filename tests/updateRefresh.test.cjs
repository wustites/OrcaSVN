const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')
const vue = require('vue')

function deferred() {
  let resolve
  const promise = new Promise(yes => { resolve = yes })
  return { promise, resolve }
}

function harness() {
  const update = deferred(), refresh = deferred(), calls = [], lifecycle = {}, timers = new Set()
  const store = {
    currentPath: '/wc', svnInfo: { revision: 1 }, statusList: [],
    setError(error) { this.error = error },
  }
  const mocks = {
    vue: {
      ...vue, watch() {}, onUnmounted() {},
      onActivated(fn) { lifecycle.activate = fn },
      onDeactivated(fn) { lifecycle.deactivate = fn },
    },
    'vue-router': { useRouter: () => ({}) },
    'vue-i18n': { useI18n: () => ({ t: key => key, locale: vue.ref('en-US') }) },
    'element-plus/es/components/message/index': { ElMessage: { success() {}, error() {} } },
    '@/stores/workspace': { useWorkspaceStore: () => store },
    '@/composables/useWorkspace': { useWorkspace: () => ({
      refreshStatusIfStale: async () => false,
      refreshStatusAfterMutation: async () => {
        calls.push('refresh')
        await refresh.promise
        store.svnInfo = store.revisionUnavailable ? null : { revision: 2 }
        store.statusList = [{ path: 'conflict.txt', status_code: 'conflicted' }]
        return true
      },
    }) },
    '@/composables/useSettings': { useSettings: () => ({ settings: {} }) },
    '@/composables/useSvnStatus': {},
    '@/api/svn': {
      svnUpdate: async () => { calls.push('update'); await update.promise },
      svnRemoteInfo: async () => { calls.push('remote'); return { revision: 2 } },
    },
  }
  const script = fs.readFileSync('src/views/UpdateView.vue', 'utf8').match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
  const code = ts.transpileModule(script + '\nexports.actions = { doUpdate, updating };', { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText
  const context = {
    exports: {}, require: name => { assert.ok(name in mocks, name); return mocks[name] },
    window: {
      setInterval(fn) { timers.add(fn); return fn },
      clearInterval(fn) { timers.delete(fn) },
    },
  }
  vm.runInNewContext(code, context)
  return { ...context.exports.actions, update, refresh, calls, store, lifecycle, timers }
}

test('leaving during an update still refreshes the shared revision and conflict list', async () => {
  const h = harness()
  h.lifecycle.activate()
  const pending = h.doUpdate()
  h.lifecycle.deactivate()
  assert.equal(h.timers.size, 0)
  h.update.resolve()
  h.refresh.resolve()
  await pending
  assert.deepEqual(h.calls, ['update', 'refresh'])
  assert.equal(h.store.svnInfo.revision, 2)
  assert.equal(h.store.statusList[0].status_code, 'conflicted')
  assert.equal(h.updating.value, false)
})

test('deactivation during the post-update refresh suppresses remote requests', async () => {
  const h = harness()
  h.lifecycle.activate()
  const pending = h.doUpdate()
  h.update.resolve()
  for (let i = 0; i < 10; i++) await Promise.resolve()
  assert.deepEqual(h.calls, ['update', 'refresh'])
  h.lifecycle.deactivate()
  h.refresh.resolve()
  await pending
  assert.deepEqual(h.calls, ['update', 'refresh'])
  assert.equal(h.store.svnInfo.revision, 2)
})

test('an active update refreshes local state once before querying the remote revision', async () => {
  const h = harness()
  h.lifecycle.activate()
  const pending = h.doUpdate()
  h.update.resolve()
  h.refresh.resolve()
  await pending
  assert.deepEqual(h.calls, ['update', 'refresh', 'remote'])
})

test('completion in a different workspace does not refresh that workspace', async () => {
  const h = harness()
  h.lifecycle.activate()
  const pending = h.doUpdate()
  h.store.currentPath = '/other'
  h.update.resolve()
  await pending
  assert.deepEqual(h.calls, ['update'])
})

test('an unknown local revision does not treat remote commits as incoming', async () => {
  const h = harness()
  h.store.revisionUnavailable = true
  h.lifecycle.activate()
  const pending = h.doUpdate()
  h.update.resolve()
  h.refresh.resolve()
  await pending
  assert.deepEqual(h.calls, ['update', 'refresh'])
})
