const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')
const vue = require('vue')
const pinia = require('pinia')

function load(file, mocks, storage) {
  const context = { exports: {}, localStorage: storage, require: name => {
    assert.ok(name in mocks, name)
    return mocks[name]
  } }
  const code = ts.transpileModule(fs.readFileSync(file, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText
  vm.runInNewContext(code, context)
  return context.exports
}

function harness(paths = ['/a', '/b', '/c'], storedValues = {}) {
  const values = new Map([['orcasvn-recent-workspaces', JSON.stringify(paths)], ...Object.entries(storedValues)])
  let failStorage = false
  let failOnceKey
  const storage = {
    getItem: key => values.get(key) ?? null,
    setItem: (key, value) => {
      if (failStorage) throw Error('存储空间不足')
      if (key === failOnceKey) { failOnceKey = undefined; throw Error('存储空间不足') }
      values.set(key, value)
    },
    removeItem: key => values.delete(key),
  }
  pinia.setActivePinia(pinia.createPinia())
  const storeModule = load('src/stores/workspace.ts', {
    pinia, vue, '@/utils/workspaceSnapshot': { invalidateWorkspaceSnapshot() {} },
  }, storage)
  const store = storeModule.useWorkspaceStore()
  return { store, values, storage, failStorage: () => { failStorage = true }, failOnceOn: key => { failOnceKey = key }, reload: () => {
    pinia.setActivePinia(pinia.createPinia())
    return storeModule.useWorkspaceStore()
  } }
}

function summaries(store) {
  const requests = [], cleanup = [], currentRefreshes = []
  const api = {
    svnStatus: path => new Promise((resolve, reject) => requests.push({ path, resolve, reject })),
    workspaceLatestModified: async () => 1700000000123,
    readGitignore: async () => null,
  }
  const module = load('src/composables/useWorkspaceSummaries.ts', {
    vue: { ...vue, onUnmounted: fn => cleanup.push(fn) },
    '@/api/svn': api,
    '@/stores/workspace': { useWorkspaceStore: () => store },
    '@/composables/useSettings': { useSettings: () => ({ settings: { gitignoreEnabled: false } }) },
    '@/composables/useWorkspace': { useWorkspace: () => ({ refreshStatusIfStale: async () => currentRefreshes.push(store.currentPath) }) },
    '@/utils/gitignore': { parseGitignore: () => [] },
    '@/utils/gitignoreWorker': { filterByGitignoreAsync: async list => list },
  })
  return { ...module.useWorkspaceSummaries(), requests, currentRefreshes, stop: () => cleanup.forEach(fn => fn()) }
}

const changes = [
  { path: 'a', status_code: 'modified', prop_status: 'modified' },
  { path: 'b', status_code: 'normal', prop_status: 'modified' },
  { path: 'c', status_code: 'missing', prop_status: 'none' },
  { path: 'd', status_code: 'unversioned', prop_status: 'none' },
]
const settle = async () => { for (let i = 0; i < 20; i++) await Promise.resolve() }

test('旧列表保留超过十个项目，切换不重排，备注保存失败不冒充成功', () => {
  const paths = Array.from({ length: 12 }, (_, i) => `/project-${i}`)
  const h = harness(paths)
  h.store.rememberWorkspace(paths[3])
  assert.deepEqual([...h.store.recentWorkspaces], paths)
  h.store.setWorkspaceNote(paths[0], '  临时修复  ')
  assert.equal(h.store.workspaceNotes[paths[0]], '临时修复')
  assert.equal(JSON.parse(h.values.get('orcasvn-workspace-notes-v1'))[paths[0]], '临时修复')
  assert.throws(() => h.store.setWorkspaceNote(paths[0], '长'.repeat(201)), /200/)
  h.failStorage()
  assert.throws(() => h.store.setWorkspaceNote(paths[0], '新备注'), /备注保存失败/)
  assert.equal(h.store.workspaceNotes[paths[0]], '临时修复')
})

test('摘要统计不重复计数，旧请求及删除后重加的请求不能覆盖结果', () => {
  const { store } = harness()
  const old = store.beginWorkspaceSummary('/a')
  const fresh = store.beginWorkspaceSummary('/a')
  store.setWorkspaceSummary('/a', changes, 1700000000123, fresh)
  store.setWorkspaceSummary('/a', [], null, old)
  assert.equal(store.workspaceSummaries['/a'].changed, 2)
  assert.equal(store.workspaceSummaries['/a'].missing, 1)
  store.setWorkspaceSummaryError('/a', '读取失败', fresh)
  assert.equal(store.workspaceSummaries['/a'].changed, 2)
  store.setWorkspaceNote('/a', '备注')
  store.removeRecentWorkspace('/a')
  store.rememberWorkspace('/a')
  store.setWorkspaceSummary('/a', changes, null, fresh)
  assert.equal(store.workspaceSummaries['/a'], undefined)
  assert.equal(store.workspaceNotes['/a'], undefined)
})

test('刷新串行扫描非当前项目，失败保持未知，当前项目复用现有刷新', async () => {
  const { store } = harness()
  store.setCurrentPath('/a', false)
  const h = summaries(store)
  const pending = h.refreshSummaries()
  await settle()
  assert.deepEqual(h.currentRefreshes, ['/a'])
  assert.deepEqual(h.requests.map(item => item.path), ['/b'])
  h.requests[0].resolve(changes)
  await settle()
  assert.deepEqual(h.requests.map(item => item.path), ['/b', '/c'])
  h.requests[1].reject(Error('路径不存在'))
  await pending
  assert.equal(store.workspaceSummaries['/b'].changed, 2)
  assert.equal(store.workspaceSummaries['/b'].latestModified, 1700000000123)
  assert.equal(store.workspaceSummaries['/c'], undefined)
  assert.match(store.workspaceSummaryErrors['/c'], /路径不存在/)
  assert.equal(h.refreshing.value, false)
  h.stop()
})

test('扫描期间切换工作区与组件卸载不会回写旧摘要或继续队列', async () => {
  const { store } = harness()
  store.setCurrentPath('/a', false)
  const h = summaries(store)
  const pending = h.refreshSummaries()
  await settle()
  store.setCurrentPath('/b', false)
  const fresh = store.beginWorkspaceSummary('/b')
  store.setWorkspaceSummary('/b', [], 1, fresh)
  h.requests[0].resolve(changes)
  await settle()
  assert.equal(store.workspaceSummaries['/b'].changed, 0)
  h.stop()
  h.requests[1].resolve(changes)
  await pending
  assert.equal(store.workspaceSummaries['/c'], undefined)
})

test('移除正在刷新的当前工作区会清除加载状态和当前文件', () => {
  const { store } = harness()
  store.setCurrentPath('/a', false)
  store.setLoading(true)
  store.setStatusList(changes)
  store.removeRecentWorkspace('/a')
  assert.equal(store.currentPath, null)
  assert.equal(store.isLoading, false)
  assert.equal(store.statusList.length, 0)
  assert.equal(store.recentWorkspaces.includes('/a'), false)
})

test('名称自然排序和修改时间排序保持独立的手动顺序，排序与折叠可恢复', () => {
  const h = harness(['/project10', '/project2', '/other/project2', '/unknown'])
  const original = [...h.store.recentWorkspaces]
  h.store.setWorkspaceSortMode('name')
  assert.deepEqual([...h.store.sortedWorkspaces], ['/project2', '/other/project2', '/project10', '/unknown'])
  h.store.setWorkspaceSummary('/project10', [], 100, h.store.beginWorkspaceSummary('/project10'))
  h.store.setWorkspaceSummary('/project2', [], 200, h.store.beginWorkspaceSummary('/project2'))
  h.store.setWorkspaceSummary('/other/project2', [], 200, h.store.beginWorkspaceSummary('/other/project2'))
  h.store.setWorkspaceSortMode('modified')
  assert.deepEqual([...h.store.sortedWorkspaces], ['/project2', '/other/project2', '/project10', '/unknown'])
  h.store.setWorkspaceSortMode('manual')
  assert.deepEqual([...h.store.sortedWorkspaces], original)
  h.store.setWorkspaceListCollapsed(true)
  const restored = h.reload()
  assert.equal(restored.workspaceSortMode, 'manual')
  assert.equal(restored.workspaceListCollapsed, true)
  assert.deepEqual([...restored.sortedWorkspaces], original)
})

test('拖动采用可见顺序，自动转手动排序并保留备注，失效目标不会破坏列表', () => {
  const h = harness(['/z', '/c', '/a'])
  h.store.setWorkspaceNote('/z', '临时代码')
  h.store.setWorkspaceSortMode('name')
  h.store.reorderWorkspaces('/z', '/a', true)
  assert.deepEqual([...h.store.recentWorkspaces], ['/a', '/z', '/c'])
  assert.equal(h.store.workspaceSortMode, 'manual')
  h.store.reorderWorkspaces('/c', '/a', false)
  h.store.reorderWorkspaces('/missing', '/a', false)
  h.store.reorderWorkspaces('/a', '/a', true)
  const restored = h.reload()
  assert.deepEqual([...restored.sortedWorkspaces], ['/c', '/a', '/z'])
  assert.equal(restored.workspaceNotes['/z'], '临时代码')
  h.failStorage()
  assert.throws(() => restored.reorderWorkspaces('/z', '/c', false), /保存失败/)
  assert.throws(() => restored.setWorkspaceSortMode('modified'), /保存失败/)
  assert.throws(() => restored.setWorkspaceListCollapsed(true), /保存失败/)
  assert.deepEqual([...restored.recentWorkspaces], ['/c', '/a', '/z'])
  assert.equal(restored.workspaceSortMode, 'manual')
  assert.equal(restored.workspaceListCollapsed, false)
})

test('批量添加去重且不切换当前工作区，取消选择保持原记录', async () => {
  const h = harness(['D:\\Projects\\A'])
  h.store.setCurrentPath('D:\\Projects\\A')
  let selection = ['d:/projects/a/', 'D:\\Projects\\B', 'D:\\Projects\\B']
  const calls = []
  const module = load('src/composables/useWorkspace.ts', {
    '@/stores/workspace': { useWorkspaceStore: () => h.store },
    '@/api/svn': { svnStatus: () => { throw Error('已有当前工作区时不能重新扫描') } },
    '@tauri-apps/plugin-dialog': { open: async options => { calls.push(options); return selection } },
    '@/i18n': { default: { global: { t: key => key } } },
    '@/composables/useSettings': {}, '@/utils/gitignore': {}, '@/utils/gitignoreWorker': {},
    '@/utils/workspaceSnapshot': {}, '@/utils/svnInfoCache': {},
  }, h.storage)
  assert.equal(await module.useWorkspace().openWorkspace('添加工作区'), true)
  assert.equal(calls[0].multiple, true)
  assert.deepEqual([...h.store.recentWorkspaces], ['D:\\Projects\\A', 'D:\\Projects\\B'])
  assert.equal(h.store.currentPath, 'D:\\Projects\\A')
  assert.equal(h.values.get('orcasvn-last-workspace'), 'D:\\Projects\\A')
  selection = null
  assert.equal(await module.useWorkspace().openWorkspace('添加工作区'), false)
  assert.deepEqual([...h.store.recentWorkspaces], ['D:\\Projects\\A', 'D:\\Projects\\B'])
})

test('空状态批量添加仅加载首个目录，失败时仍保留全部记录及错误', async () => {
  const h = harness([]), scanned = []
  const module = load('src/composables/useWorkspace.ts', {
    '@/stores/workspace': { useWorkspaceStore: () => h.store },
    '@/api/svn': {
      svnStatus: async path => { scanned.push(path); throw Error('不是 SVN 工作区') },
      svnLocalRevision: async () => 12,
    },
    '@tauri-apps/plugin-dialog': { open: async () => ['/first', '/second'] },
    '@/i18n': { default: { global: { t: key => key } } },
    '@/composables/useSettings': { useSettings: () => ({ settings: { gitignoreEnabled: false } }) },
    '@/utils/gitignore': {}, '@/utils/gitignoreWorker': {},
    '@/utils/workspaceSnapshot': { getWorkspaceSnapshot: () => null },
    '@/utils/svnInfoCache': { getCachedSvnInfoMetadata: () => ({ url: 'file:///repo' }) },
  }, h.storage)
  assert.equal(await module.useWorkspace().openWorkspace('添加工作区'), false)
  assert.deepEqual(scanned, ['/first'])
  assert.deepEqual([...h.store.recentWorkspaces], ['/first', '/second'])
  assert.match(h.store.workspaceSummaryErrors['/first'], /不是 SVN 工作区/)
  assert.equal(h.store.currentPath, null)
})

test('已有 Windows 路径的别名加载沿用原键，不丢失选中状态或摘要', async () => {
  const savedPath = 'D:\\Projects\\A'
  const h = harness([savedPath]), scanned = []
  const module = load('src/composables/useWorkspace.ts', {
    '@/stores/workspace': { useWorkspaceStore: () => h.store },
    '@/api/svn': {
      svnStatus: async path => { scanned.push(path); return changes },
      svnLocalRevision: async () => 12,
      workspaceLatestModified: async () => 1700000000123,
    },
    '@tauri-apps/plugin-dialog': {},
    '@/i18n': { default: { global: { t: key => key } } },
    '@/composables/useSettings': { useSettings: () => ({ settings: { gitignoreEnabled: false } }) },
    '@/utils/gitignore': {},
    '@/utils/gitignoreWorker': { filterByGitignoreAsync: async list => list },
    '@/utils/workspaceSnapshot': { getWorkspaceSnapshot: () => null, cacheWorkspaceSnapshot() {} },
    '@/utils/svnInfoCache': { getCachedSvnInfoMetadata: () => ({ url: 'file:///repo' }) },
  }, h.storage)
  assert.equal(await module.useWorkspace().loadWorkspace('d:/projects/a/'), true)
  await settle()
  assert.deepEqual(scanned, [savedPath])
  assert.deepEqual([...h.store.recentWorkspaces], [savedPath])
  assert.equal(h.store.currentPath, savedPath)
  assert.equal(h.values.get('orcasvn-last-workspace'), savedPath)
  assert.equal(h.store.workspaceSummaries[savedPath].changed, 2)
  assert.equal(h.store.workspaceSummaries[savedPath].latestModified, 1700000000123)
  assert.equal(h.store.resolveWorkspacePath('D:/Projects/NEW'), 'D:/Projects/NEW')
  assert.deepEqual([...h.store.recentWorkspaces], [savedPath])
})

test('收藏沿用已保存的目录键并可重载，取消收藏不改变工作区及当前内容', () => {
  const a = 'D:\\Projects\\A', b = 'D:\\Projects\\B'
  const h = harness([b, a, '/unix'], {
    'orcasvn-favorite-workspaces-v1': JSON.stringify(['d:/projects/a/', a, '/removed', null, 12, '/UNIX']),
  })
  assert.deepEqual([...h.store.favoriteWorkspaces], [a])
  assert.equal(h.store.isWorkspaceFavorite('D:/PROJECTS/A/'), true)
  h.store.setWorkspaceFavorite('d:/projects/b/', true)
  h.store.setWorkspaceFavorite(b, true)
  assert.deepEqual(JSON.parse(h.values.get('orcasvn-favorite-workspaces-v1')), [a, b])
  const store = h.reload()
  assert.deepEqual([...store.sortedFavoriteWorkspaces], [b, a])
  store.setWorkspaceSortMode('name')
  assert.deepEqual([...store.sortedFavoriteWorkspaces], [a, b])
  store.setCurrentPath(a)
  store.setWorkspaceNote(a, '常用工作区')
  store.setStatusList(changes)
  store.setWorkspaceFavorite('d:/projects/a/', false)
  assert.deepEqual([...store.sortedFavoriteWorkspaces], [b])
  assert.deepEqual([...store.recentWorkspaces], [b, a, '/unix'])
  assert.equal(store.currentPath, a)
  assert.equal(store.workspaceNotes[a], '常用工作区')
  assert.equal(store.statusList.length, changes.length)
})

test('移除工作区同步清理收藏，重加该目录时不会恢复旧收藏', () => {
  const h = harness()
  h.store.setWorkspaceFavorite('/a', true)
  h.store.setWorkspaceFavorite('/b', true)
  h.store.setCurrentPath('/a')
  h.store.removeRecentWorkspace('/a')
  assert.deepEqual([...h.store.favoriteWorkspaces], ['/b'])
  assert.deepEqual(JSON.parse(h.values.get('orcasvn-favorite-workspaces-v1')), ['/b'])
  assert.equal(h.store.currentPath, null)
  assert.equal(h.values.has('orcasvn-last-workspace'), false)
  h.store.registerWorkspaces(['/a'])
  const restored = h.reload()
  assert.equal(restored.isWorkspaceFavorite('/a'), false)
  assert.deepEqual([...restored.sortedFavoriteWorkspaces], ['/b'])
})

test('收藏和移除保存失败时保留原状态，中途写入失败可回滚已保存的数据', () => {
  const h = harness()
  h.store.setWorkspaceFavorite('/a', true)
  h.store.setCurrentPath('/a')
  h.store.setWorkspaceNote('/a', '备注')
  h.store.setWorkspaceSummary('/a', changes, 100, h.store.beginWorkspaceSummary('/a'))
  const previous = new Map(h.values)
  h.failOnceOn('orcasvn-favorite-workspaces-v1')
  assert.throws(() => h.store.setWorkspaceFavorite('/a', false), /收藏保存失败/)
  assert.equal(h.store.isWorkspaceFavorite('/a'), true)
  assert.deepEqual(h.values, previous)
  h.failOnceOn('orcasvn-favorite-workspaces-v1')
  assert.throws(() => h.store.removeRecentWorkspace('/a'), /工作区移除失败/)
  assert.deepEqual([...h.store.recentWorkspaces], ['/a', '/b', '/c'])
  assert.equal(h.store.isWorkspaceFavorite('/a'), true)
  assert.equal(h.store.currentPath, '/a')
  assert.equal(h.store.workspaceNotes['/a'], '备注')
  assert.equal(h.store.workspaceSummaries['/a'].changed, 2)
  assert.deepEqual(h.values, previous)
})

test('损坏的收藏记录不影响全部工作区，收藏未知目录不会新增工作区', () => {
  for (const stored of ['{broken', '{}', 'null']) {
    const h = harness(['/a', '/b'], { 'orcasvn-favorite-workspaces-v1': stored })
    assert.deepEqual([...h.store.favoriteWorkspaces], [])
    assert.deepEqual([...h.store.sortedWorkspaces], ['/a', '/b'])
    assert.throws(() => h.store.setWorkspaceFavorite('/missing', true), /工作区已移除/)
    assert.deepEqual([...h.store.recentWorkspaces], ['/a', '/b'])
  }
})
