const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const ts = require('typescript')
const vm = require('node:vm')
const vue = require('vue')

function deferred() {
  let resolve, reject
  const promise = new Promise((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
function harness(status = 'modified', path = 'file.ts') {
  const confirmation = deferred(), dialogs = [], operations = []
  const file = { path, status_code: status, prop_status: 'none' }
  const store = vue.reactive({ currentPath: '/wc', statusList: [file], error: null, setError(error) { this.error = error } })
  let refreshes = 0, operation = async () => {}, unmount
  const mocks = {
    vue: { ...vue, onMounted() {}, onBeforeUnmount(fn) { unmount = fn } },
    'vue-router': { useRouter: () => ({}) },
    'vue-i18n': { useI18n: () => ({ t: key => key }) },
    '@/stores/workspace': { useWorkspaceStore: () => store },
    '@/api/svn': {
      svnRevert: async (...args) => { operations.push({ type: 'revert', args }); await operation() },
      deleteUnversioned: async (...args) => { operations.push({ type: 'delete', args }); await operation() },
    },
    'element-plus/es/components/message/index': {},
    'element-plus/es/components/message-box/index': { ElMessageBox: { confirm: (...args) => { dialogs.push(args); return confirmation.promise } } },
    '@/composables/useSvnStatus': {},
    '@/composables/useWorkspace': { useWorkspace: () => ({ refreshStatusAfterMutation: async () => { refreshes++; return true } }) },
    '@/components/VirtualViewport.vue': {},
    '@/components/DiffViewer.vue': {},
    '@/components/GlassLoading.vue': {},
    '@/utils/clipboard': {},
    '@/utils/statusSelection': { retainedSelection: (selected, paths) => selected && paths.includes(selected) ? selected : null },
  }
  const script = fs.readFileSync('src/views/WorkspaceView.vue', 'utf8').match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1]
  const code = ts.transpileModule(script + '\nexports.actions = { handleFileAction, fileActionLabel, pendingFileAction };', { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText
  const context = { exports: {}, document: { removeEventListener() {} }, require: name => { assert.ok(name in mocks, name); return mocks[name] } }
  vm.runInNewContext(code, context)
  return { ...context.exports.actions, file, store, confirmation, dialogs, operations,
    refreshes: () => refreshes, setOperation: fn => { operation = fn }, unmount: () => unmount() }
}

test('revert waits for explicit confirmation and displays the target as escaped text', async () => {
  const h = harness('modified', '<img src=x onerror=alert(1)>.ts')
  const pending = h.handleFileAction(h.file)
  assert.equal(h.operations.length, 0)
  assert.equal(h.dialogs.length, 1)
  const [message, title, options] = h.dialogs[0]
  assert.equal(title, 'common.revert')
  assert.equal(message.children[0].children, 'workspace.revertFileConfirm')
  assert.equal(message.children[1].children, '/wc/<img src=x onerror=alert(1)>.ts')
  assert.equal(message.children[1].props.innerHTML, undefined)
  assert.equal(options.closeOnClickModal, false)
  assert.equal(options.autofocus, false)
  h.confirmation.resolve('confirm')
  await pending
  assert.equal(h.operations[0].type, 'revert')
  assert.equal(h.operations[0].args[0], '/wc')
  assert.equal(Array.from(h.operations[0].args[1])[0], h.file.path)
  assert.equal(h.refreshes(), 1)
  assert.equal(h.pendingFileAction.value, null)
})

test('unversioned targets use Delete labels and the permanent deletion warning', async () => {
  const h = harness('unversioned', 'new-directory')
  assert.equal(h.fileActionLabel(h.file), 'common.delete')
  const pending = h.handleFileAction(h.file)
  assert.equal(h.dialogs[0][0].children[0].children, 'workspace.deleteUnversionedConfirm')
  assert.equal(h.dialogs[0][2].confirmButtonText, 'common.delete')
  h.confirmation.resolve('confirm')
  await pending
  assert.equal(h.operations[0].type, 'delete')
})

test('cancel and close leave files and error state unchanged', async () => {
  for (const action of ['cancel', 'close']) {
    const h = harness()
    const pending = h.handleFileAction(h.file)
    h.confirmation.reject(action)
    await pending
    assert.equal(h.operations.length, 0)
    assert.equal(h.refreshes(), 0)
    assert.equal(h.store.error, null)
    assert.equal(h.pendingFileAction.value, null)
  }
})

test('a workspace switch, even away and back, invalidates the confirmation', async () => {
  const h = harness()
  const pending = h.handleFileAction(h.file)
  h.store.currentPath = '/another'
  h.store.currentPath = '/wc'
  h.confirmation.resolve('confirm')
  await pending
  assert.equal(h.operations.length, 0)
})

test('removed or changed SVN status requires a new confirmation', async () => {
  for (const change of [h => { h.store.statusList = [] }, h => { h.store.statusList[0].status_code = 'added' }]) {
    const h = harness('unversioned')
    const pending = h.handleFileAction(h.file)
    change(h)
    h.confirmation.resolve('confirm')
    await pending
    assert.equal(h.operations.length, 0)
  }
})

test('duplicate clicks cannot open multiple dialogs or execute the operation twice', async () => {
  const h = harness(), execution = deferred()
  h.setOperation(() => execution.promise)
  const first = h.handleFileAction(h.file)
  await h.handleFileAction(h.file)
  assert.equal(h.dialogs.length, 1)
  h.confirmation.resolve('confirm')
  for (let i = 0; i < 5; i++) await Promise.resolve()
  await h.handleFileAction(h.file)
  assert.equal(h.operations.length, 1)
  execution.resolve()
  await first
  assert.equal(h.pendingFileAction.value, null)
})

test('operation failures retain their error and unlock file actions', async () => {
  const h = harness()
  h.setOperation(async () => { throw new Error('SVN failed') })
  const pending = h.handleFileAction(h.file)
  h.confirmation.resolve('confirm')
  await pending
  assert.equal(h.store.error, 'Error: SVN failed')
  assert.equal(h.refreshes(), 0)
  assert.equal(h.pendingFileAction.value, null)
})

test('unmounting the view invalidates a pending confirmation', async () => {
  const h = harness()
  const pending = h.handleFileAction(h.file)
  h.unmount()
  h.confirmation.resolve('confirm')
  await pending
  assert.equal(h.operations.length, 0)
})


test('unverified cached status cannot start or confirm a destructive file action', async () => {
  const h = harness()
  h.store.statusIsStale = true
  await h.handleFileAction(h.file)
  assert.equal(h.dialogs.length, 0)
  h.store.statusIsStale = false
  const pending = h.handleFileAction(h.file)
  h.store.statusIsStale = true
  h.confirmation.resolve('confirm')
  await pending
  assert.equal(h.operations.length, 0)
})
