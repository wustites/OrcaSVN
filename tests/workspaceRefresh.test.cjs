const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')

function deferred() {
  let resolve, reject
  const promise = new Promise((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
function harness() {
  const requests = [], snapshots = new Map(), writes = [], invalidations = []
  const store = {
    currentPath: '/a', statusList: [], svnInfo: null, gitignorePatterns: [],
    setCurrentPath(path) { this.currentPath = path },
    setStatusList(list) { this.statusList = list },
    setSvnInfo(info) { this.svnInfo = info },
    setStatusIsStale(value) { this.statusIsStale = value },
    setLoading(value) { this.isLoading = value },
    setError(value) { this.error = value },
    setGitignorePatterns(value) { this.gitignorePatterns = value },
    setGitignoreMtime() {}, setGitignoreWorkspacePath() {}, rememberWorkspace() {},
    lastWorkspacePath: null,
    getLastWorkspacePath() { return this.lastWorkspacePath },
    clearWorkspace() { this.lastWorkspacePath = null; this.resetWorkspaceState() },
    resetWorkspaceState() { this.currentPath = null; this.error = null },
  }
  let filter = async list => list
  const mocks = {
    '@/stores/workspace': { useWorkspaceStore: () => store },
    '@/api/svn': {
      svnStatus: (path, force = false) => { const request = { path, force, ...deferred() }; requests.push(request); return request.promise },
      svnLocalRevision: async () => 12,
    },
    '@tauri-apps/plugin-dialog': {},
    '@/composables/useSettings': { useSettings: () => ({ settings: { gitignoreEnabled: false } }) },
    '@/i18n': { default: { global: { t: key => key } } },
    '@/utils/gitignore': {},
    '@/utils/gitignoreWorker': { filterByGitignoreAsync: list => filter(list) },
    '@/utils/workspaceSnapshot': { getWorkspaceSnapshot: path => snapshots.get(path) || null, cacheWorkspaceSnapshot: (...args) => writes.push(args), invalidateWorkspaceSnapshot: path => { invalidations.push(path); snapshots.delete(path) } },
    '@/utils/svnInfoCache': { getCachedSvnInfoMetadata: () => ({ url: 'file:///repo' }) },
  }
  const context = { exports: {}, require: name => { assert.ok(name in mocks, name); return mocks[name] } }
  const code = ts.transpileModule(fs.readFileSync('src/composables/useWorkspace.ts', 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText
  vm.runInNewContext(code, context)
  return { store, requests, snapshots, writes, invalidations, useWorkspace: context.exports.useWorkspace, setFilter: value => { filter = value } }
}

test('separate consumers share an in-flight refresh and background polls reuse fresh state', async () => {
  const h = harness(), a = h.useWorkspace(), b = h.useWorkspace()
  const first = a.refreshStatus(), second = b.refreshStatus()
  assert.equal(first, second)
  assert.equal(h.requests.length, 1)
  h.requests[0].resolve([{ path: 'first' }])
  assert.equal(await first, true)
  await b.refreshStatusIfStale(60_000)
  assert.equal(h.requests.length, 1)
  const manual = a.refreshStatus()
  assert.equal(h.requests.length, 2)
  h.requests[1].resolve([{ path: 'manual' }])
  assert.equal(await manual, true)
})

test('failed refresh is retried rather than treated as fresh', async () => {
  const h = harness(), workspace = h.useWorkspace()
  const failed = workspace.refreshStatus()
  h.requests[0].reject(new Error('status failed'))
  assert.equal(await failed, false)
  const retry = workspace.refreshStatusIfStale(60_000)
  assert.equal(h.requests.length, 2)
  h.requests[1].resolve([])
  assert.equal(await retry, true)
})

test('post-mutation scan supersedes an earlier background scan', async () => {
  const h = harness(), workspace = h.useWorkspace()
  const background = workspace.refreshStatus()
  const mutation = workspace.refreshStatusAfterMutation()
  assert.equal(h.requests.length, 2)
  h.requests[1].resolve([{ path: 'new' }])
  assert.equal(await mutation, true)
  h.requests[0].resolve([{ path: 'old' }])
  assert.equal(await background, false)
  assert.equal(h.store.statusList[0].path, 'new')
  assert.equal(h.store.isLoading, false)
})

test('workspace switch rejects late results from the previous workspace', async () => {
  const h = harness(), workspace = h.useWorkspace()
  const previous = workspace.refreshStatus()
  const current = workspace.loadWorkspace('/b')
  assert.equal(h.requests[1].path, '/b')
  h.requests[1].resolve([{ path: 'b' }])
  assert.equal(await current, true)
  h.requests[0].resolve([{ path: 'a' }])
  assert.equal(await previous, false)
  assert.equal(h.store.statusList[0].path, 'b')
})

test('late worker reply cannot overwrite status after a mutation', async () => {
  const h = harness(), workspace = h.useWorkspace(), worker = deferred()
  h.setFilter(list => list[0]?.path === 'old' ? worker.promise : Promise.resolve(list))
  const old = workspace.refreshStatus()
  h.requests[0].resolve([{ path: 'old' }])
  // Let the old request reach the asynchronous filter.
  for (let i = 0; i < 10; i++) await Promise.resolve()
  const fresh = workspace.refreshStatusAfterMutation()
  h.requests[1].resolve([{ path: 'fresh' }])
  assert.equal(await fresh, true)
  worker.resolve([{ path: 'old' }])
  assert.equal(await old, false)
  assert.equal(h.store.statusList[0].path, 'fresh')
})


test('reopening shows a cached small change list synchronously and verifies it in the background', async () => {
  const h = harness(), workspace = h.useWorkspace()
  h.snapshots.set('/b', { statusList: [{ path: 'cached.ts' }], svnInfo: { revision: 11 } })
  const pending = workspace.loadWorkspace('/b')
  assert.equal(h.store.currentPath, '/b')
  assert.equal(h.store.statusList[0].path, 'cached.ts')
  assert.equal(h.store.svnInfo.revision, 11)
  assert.equal(h.store.statusIsStale, true)
  assert.equal(h.store.isLoading, true)
  assert.equal(h.requests.length, 1)
  h.requests[0].resolve([{ path: 'fresh.ts' }])
  assert.equal(await pending, true)
  assert.equal(h.store.statusList[0].path, 'fresh.ts')
  assert.equal(h.store.statusIsStale, false)
  assert.equal(h.store.isLoading, false)
  assert.equal(h.writes[0][0], '/b')
  assert.equal(h.writes[0][1][0].path, 'fresh.ts')
})

test('an empty cached list is still marked unverified until the scan succeeds', async () => {
  const h = harness()
  h.snapshots.set('/b', { statusList: [], svnInfo: { revision: 11 } })
  const pending = h.useWorkspace().loadWorkspace('/b')
  assert.equal(h.store.statusIsStale, true)
  h.requests[0].resolve([{ path: 'new.ts' }])
  assert.equal(await pending, true)
  assert.equal(h.store.statusIsStale, false)
})

test('failed verification preserves the cached view but does not mark it fresh', async () => {
  const h = harness(), workspace = h.useWorkspace()
  h.snapshots.set('/b', { statusList: [{ path: 'cached.ts' }], svnInfo: { revision: 11 } })
  const pending = workspace.loadWorkspace('/b')
  h.requests[0].reject(new Error('workspace unavailable'))
  assert.equal(await pending, false)
  assert.equal(h.store.currentPath, '/b')
  assert.equal(h.store.statusList[0].path, 'cached.ts')
  assert.equal(h.store.statusIsStale, true)
  assert.match(h.store.error, /workspace unavailable/)
  assert.equal(h.writes.length, 0)
  const retry = workspace.refreshStatus()
  h.requests[1].resolve([])
  assert.equal(await retry, true)
  assert.equal(h.store.statusIsStale, false)
})

test('post-mutation scan invalidates persistence and old file actions until fresh status arrives', async () => {
  const h = harness()
  h.snapshots.set('/a', { statusList: [{ path: 'old.ts' }], svnInfo: { revision: 11 } })
  const pending = h.useWorkspace().refreshStatusAfterMutation()
  assert.deepEqual(h.invalidations, ['/a'])
  assert.equal(h.snapshots.has('/a'), false)
  assert.equal(h.store.statusIsStale, true)
  h.requests[0].resolve([])
  await pending
  assert.equal(h.store.statusIsStale, false)
})

test('filesystem-scoped background checks do not satisfy an explicit complete refresh', async () => {
  const h = harness(), workspace = h.useWorkspace()
  const background = workspace.refreshStatusIfStale(0)
  assert.equal(h.requests[0].force, false)
  const manual = workspace.refreshStatus()
  assert.equal(h.requests[1].force, true)
  h.requests[1].resolve([{ path: 'verified.ts' }])
  assert.equal(await manual, true)
  h.requests[0].resolve([{ path: 'old.ts' }])
  assert.equal(await background, false)
  assert.equal(h.store.statusList[0].path, 'verified.ts')
})

test('a background refresh reports a failure once, so a dismissed error stays dismissed', async () => {
  const h = harness(), workspace = h.useWorkspace()
  const first = workspace.refreshStatusIfStale(0)
  h.requests[0].reject(new Error('offline'))
  assert.equal(await first, false)
  assert.match(h.store.error, /offline/)

  h.store.setError(null)
  const repeated = workspace.refreshStatusIfStale(0)
  h.requests[1].reject(new Error('offline'))
  assert.equal(await repeated, false)
  assert.equal(h.store.error, null)

  const changed = workspace.refreshStatusIfStale(0)
  h.requests[2].reject(new Error('denied'))
  assert.equal(await changed, false)
  assert.match(h.store.error, /denied/)
})

test('a background refresh leaves other errors alone and clears only its own on recovery', async () => {
  const h = harness(), workspace = h.useWorkspace()
  h.store.setError('cleanup failed')
  const unrelated = workspace.refreshStatusIfStale(0)
  assert.equal(h.store.error, 'cleanup failed')
  h.requests[0].resolve([])
  assert.equal(await unrelated, true)
  assert.equal(h.store.error, 'cleanup failed')

  const failed = workspace.refreshStatus()
  h.requests[1].reject(new Error('offline'))
  await failed
  const recovered = workspace.refreshStatusIfStale(0)
  h.requests[2].resolve([])
  assert.equal(await recovered, true)
  assert.equal(h.store.error, null)
})

test('loading a workspace does not join a background refresh that hides its errors', async () => {
  const h = harness(), workspace = h.useWorkspace()
  const background = workspace.refreshStatusIfStale(0)
  const load = workspace.loadWorkspace('/a')
  assert.equal(h.requests.length, 2)
  h.requests[1].reject(new Error('offline'))
  assert.equal(await load, false)
  assert.match(h.store.error, /offline/)
  h.requests[0].resolve([])
  assert.equal(await background, false)
})

test('a failed first load keeps the remembered workspace', async () => {
  const h = harness(), workspace = h.useWorkspace()
  h.store.currentPath = null
  h.store.lastWorkspacePath = '/last'
  const pending = workspace.loadWorkspace('/b')
  h.requests[0].reject(new Error('offline'))
  assert.equal(await pending, false)
  assert.equal(h.store.currentPath, null)
  assert.equal(h.store.lastWorkspacePath, '/last')
  assert.match(h.store.error, /offline/)
})

test('a failed launch path stays reported after falling back to the last workspace', async () => {
  const h = harness(), workspace = h.useWorkspace()
  h.store.currentPath = null
  h.store.lastWorkspacePath = '/last'
  const pending = workspace.openStartupWorkspace('/launch')
  h.requests[0].reject(new Error('launch failed'))
  for (let i = 0; i < 20 && h.requests.length < 2; i++) await new Promise(resolve => setImmediate(resolve))
  assert.equal(h.requests[1].path, '/last')
  assert.equal(h.store.error, null)
  h.requests[1].resolve([])
  await pending
  assert.equal(h.store.currentPath, '/last')
  assert.match(h.store.error, /launch failed/)
})

test('a successful poll of the fallback workspace keeps the failed launch path error', async () => {
  const h = harness(), workspace = h.useWorkspace()
  h.store.currentPath = null
  h.store.lastWorkspacePath = '/last'
  const pending = workspace.openStartupWorkspace('/launch')
  h.requests[0].reject(new Error('launch failed'))
  for (let i = 0; i < 20 && h.requests.length < 2; i++) await new Promise(resolve => setImmediate(resolve))
  h.requests[1].resolve([])
  await pending
  const poll = workspace.refreshStatusIfStale(0)
  h.requests[2].resolve([])
  assert.equal(await poll, true)
  assert.match(h.store.error, /launch failed/)
})
