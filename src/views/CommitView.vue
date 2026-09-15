<template>
  <div class="commit-view">
    <el-card class="commit-card animate-fade-in">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon><Upload /></el-icon>
            {{ $t('commit.title') }}
          </span>
        </div>
      </template>

      <div v-if="!workspaceStore.currentPath" class="no-workspace">
        <el-empty :description="$t('log.openWorkspaceFirst')">
          <el-button type="primary" @click="openWorkspace">
            <el-icon><FolderOpened /></el-icon>
            {{ $t('common.open') }}
          </el-button>
        </el-empty>
      </div>

      <div v-else class="commit-content">
        <el-alert
          :title="$t('commit.commitMessage')"
          type="info"
          :closable="false"
          class="commit-info"
          show-icon
        >
          <template #default>
            <div class="info-content">
              <div class="info-item">
                <span class="info-label">{{ $t('commit.currentWorkspace') }}：</span>
                <span class="info-value path-text">{{ workspaceStore.currentPath }}</span>
              </div>
              <div class="info-item">
                <span class="info-label">{{ $t('commit.changedFiles') }}：</span>
                <el-tag type="primary" size="small">
                  {{ $t('commit.filesCount', { count: changedFiles.length }) }}
                </el-tag>
              </div>
            </div>
          </template>
        </el-alert>

        <el-form class="commit-form" label-position="top" :disabled="loading">
          <div class="commit-toolbar">
            <el-input
              v-model="searchQuery"
              :placeholder="$t('commit.searchFiles')"
              clearable
              class="search-input"
              size="small"
            >
              <template #prefix>
                <el-icon><Search /></el-icon>
              </template>
            </el-input>
            <div class="filter-tags">
              <el-tag
                :type="filterMode === 'all' ? 'primary' : 'info'"
                effect="plain"
                size="small"
                style="cursor:pointer"
                @click="filterMode = 'all'"
              >
                {{ $t('common.all') }}
              </el-tag>
              <el-tag
                :type="filterMode === 'committable' ? 'primary' : 'info'"
                effect="plain"
                size="small"
                style="cursor:pointer"
                @click="filterMode = 'committable'"
              >
                {{ $t('commit.committable') }}
              </el-tag>
              <el-tag
                :type="filterMode === 'unversioned' ? 'primary' : 'info'"
                effect="plain"
                size="small"
                style="cursor:pointer"
                @click="filterMode = 'unversioned'"
              >
                {{ $t('status.unversioned') }}
              </el-tag>
            </div>
          </div>

          <el-form-item class="file-selection">
            <div class="commit-scope">
              <span>{{ $t('commit.scope') }}: {{ scope || workspaceStore.currentPath }}</span>
              <el-button v-if="scope" text @click="setScope('')">{{ $t('commit.allDirectories') }}</el-button>
              <strong aria-live="polite">{{ $t(hiddenSelectedCount > 0 ? 'commit.selectedCountWithHidden' : 'commit.selectedCount', { count: selectedFiles.length, hidden: hiddenSelectedCount }) }}</strong>
            </div>
            <el-tree
              class="commit-tree"
              :data="treeData"
              node-key="path"
              default-expand-all
              :expand-on-click-node="false"
              :filter-node-method="filterTreeNode"
              ref="commitTree"
            >
              <template #default="{ data }">
                <div class="commit-tree-row">
                  <el-checkbox
                    :model-value="isChecked(data)"
                    :indeterminate="isPartial(data)"
                    :disabled="loading"
                    :aria-label="data.path"
                    @click.stop
                    @change="toggleNode(data, Boolean($event))"
                  />
                  <span class="tree-name" :title="data.path">{{ data.label }}</span>
                  <span v-if="data.entry" class="status-badge" :class="getStatusClass(data.entry.status_code)">
                    {{ $t(getStatusLabelKey(data.entry.status_code)) }}
                  </span>
                  <span v-if="data.children.length" class="tree-count">{{ data.targets.length }}</span>
                  <el-button v-if="data.entry" text size="small" :disabled="loading" @click.stop="viewDiff(data.entry.path)">
                    {{ $t('common.diff') }}
                  </el-button>
                </div>
              </template>
            </el-tree>
          </el-form-item>

          <el-form-item :label="$t('commit.commitMessage')" required class="message-input">
            <el-input
              v-model="commitMessage"
              type="textarea"
              :rows="5"
              :placeholder="$t('commit.enterCommitMessage')"
              resize="vertical"
              class="commit-textarea"
            />
            <div class="char-count">
              <span :class="{ 'warning': commitMessage.length > 500 }">{{ commitMessage.length }}</span>
              <span>/ 500</span>
            </div>
            <div v-if="recentMessages.length > 0" class="recent-messages">
              <div class="recent-messages-header">
                <span class="recent-messages-title">
                  <el-icon><Calendar /></el-icon>
                  {{ $t('commit.recentMessages') }}
                </span>
                <el-button
                  text
                  size="small"
                  :aria-label="$t('commit.refreshRecentMessages')"
                  @click="loadRecentMessages"
                >
                  <el-icon><Refresh /></el-icon>
                </el-button>
              </div>
              <div class="recent-message-list">
                <button
                  v-for="entry in recentMessages"
                  :key="entry.id"
                  type="button"
                  class="recent-message-item"
                  :title="entry.message"
                  @click="useRecentMessage(entry.message)"
                >
                  <span class="recent-message-revision">●</span>
                  <span class="recent-message-text">{{ entry.message }}</span>
                </button>
              </div>
            </div>
          </el-form-item>

          <el-form-item class="form-actions">
            <el-button
              type="primary"
              @click="doCommit"
              :loading="loading"
              :disabled="!commitMessage || selectedFiles.length === 0"
            >
              <el-icon><Upload /></el-icon>
              {{ $t(hasUnversionedSelection ? 'commit.addAndReview' : 'common.commit') }}
            </el-button>
            <el-button @click="resetForm">
              <el-icon><RefreshLeft /></el-icon>
              {{ $t('common.reset') }}
            </el-button>
          </el-form-item>
        </el-form>

        <div v-if="output" class="output-area animate-fade-in">
          <div class="output-header">
            <span class="output-title">
              <el-icon><Document /></el-icon>
              {{ $t('commit.output') }}
            </span>
          </div>
          <el-input
            v-model="output"
            type="textarea"
            :rows="8"
            readonly
            class="output-textarea"
          />
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ElTree } from 'element-plus/es/components/tree/index'
import { ElCheckbox } from 'element-plus/es/components/checkbox/index'
import { ref, computed, nextTick, onMounted, watch } from 'vue'
import { useRouter, useRoute, onBeforeRouteLeave } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnAdd, svnCommit } from '@/api/svn'
import { useI18n } from 'vue-i18n'
import { getStatusClass, getStatusLabelKey } from '@/composables/useSvnStatus'
import { useWorkspace } from '@/composables/useWorkspace'
import { buildChangeTree, isWithinDirectory, type ChangeNode } from '@/utils/changeTree'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog, refreshStatus } = useWorkspace()

const RECENT_COMMIT_MESSAGES_KEY = 'orcasvn-recent-commit-messages'
interface RecentCommitMessage {
  id: string
  message: string
}

const commitTree = ref()
const selectedFiles = ref<string[]>([])
const commitMessage = ref('')
const loading = ref(false)
const output = ref('')
const recentMessages = ref<RecentCommitMessage[]>([])

const committableStatuses = new Set(['added', 'modified', 'deleted', 'replaced', 'unversioned'])

const searchQuery = ref('')
const filterMode = ref<'all' | 'committable' | 'unversioned'>('all')

const scope = computed(() => typeof route.query.directory === 'string' ? route.query.directory : '')
const allChangedFiles = computed(() => {
  return workspaceStore.statusList.filter(
    s => isWithinDirectory(s.path, scope.value) && s.status_code !== 'conflicted' && s.prop_status !== 'conflicted' && (committableStatuses.has(s.status_code) || s.prop_status === 'modified')
  )
})

const changedFiles = computed(() => {
  let files = [...allChangedFiles.value]

  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    files = files.filter(f => f.path.toLowerCase().includes(q))
  }

  if (filterMode.value === 'committable') {
    files = files.filter(f => f.status_code !== 'unversioned')
  } else if (filterMode.value === 'unversioned') {
    files = files.filter(f => f.status_code === 'unversioned')
  }

  files.sort((a, b) => {
    const aUntracked = a.status_code === 'unversioned' ? 1 : 0
    const bUntracked = b.status_code === 'unversioned' ? 1 : 0
    if (aUntracked !== bUntracked) return aUntracked - bUntracked
    return a.path.localeCompare(b.path)
  })

  return files
})

const treeData = computed(() => buildChangeTree(allChangedFiles.value, scope.value))
const selectedSet = computed(() => new Set(selectedFiles.value))
const isChecked = (node: ChangeNode) => node.targets.every(path => selectedSet.value.has(path))
const isPartial = (node: ChangeNode) => !isChecked(node) && node.targets.some(path => selectedSet.value.has(path))
const toggleNode = (node: ChangeNode, checked: boolean) => {
  const selection = new Set(selectedFiles.value)
  node.targets.forEach(path => checked ? selection.add(path) : selection.delete(path))
  selectedFiles.value = [...selection]
}
const visiblePaths = computed(() => new Set(changedFiles.value.map(file => file.path)))
const hiddenSelectedCount = computed(() => selectedFiles.value.filter(path => !visiblePaths.value.has(path)).length)
const filterTreeNode = (_value: unknown, node: unknown) => (node as ChangeNode).targets.some(path => visiblePaths.value.has(path))
watch([changedFiles, treeData], async () => {
  await nextTick()
  commitTree.value?.filter(searchQuery.value)
}, { flush: 'post' })
const setScope = (directory: string) => {
  router.replace({ name: 'commit', query: directory ? { directory } : {} })
}
const hasUnversionedSelection = computed(() => allChangedFiles.value.some(file =>
  file.status_code === 'unversioned' && selectedSet.value.has(file.path)))

const routeSelectedFiles = computed(() => {
  const files = route.query.files
  if (Array.isArray(files)) {
    return files.filter((file): file is string => typeof file === 'string')
  }
  return typeof files === 'string' ? [files] : []
})

const applyRouteSelection = () => {
  const requested = new Set(routeSelectedFiles.value)
  selectedFiles.value = allChangedFiles.value
    .filter(file => scope.value ? file.status_code !== 'unversioned' : requested.has(file.path))
    .map(file => file.path)
}
watch([scope, () => route.query.files], applyRouteSelection, { immediate: true })
watch(allChangedFiles, () => {
  const available = new Set(allChangedFiles.value.map(file => file.path))
  selectedFiles.value = selectedFiles.value.filter(path => available.has(path))
})
watch(() => workspaceStore.currentPath, () => {
  selectedFiles.value = []
  commitMessage.value = ''
  recentMessages.value = []
  sessionStorage.removeItem('orca_commit_form')
  setScope('')
  void loadRecentMessages()
})
onMounted(() => {
  const saved = sessionStorage.getItem('orca_commit_form')
  if (saved) {
    try {
      const data = JSON.parse(saved)
      if (data.workspace === workspaceStore.currentPath && data.scope === scope.value) {
        const available = new Set(allChangedFiles.value.map(file => file.path))
        selectedFiles.value = Array.isArray(data.selectedFiles)
          ? data.selectedFiles.filter((path: string) => available.has(path)) : []
        commitMessage.value = typeof data.commitMessage === 'string' ? data.commitMessage : ''
      }
    } catch { /* ignore invalid saved state */ }
    sessionStorage.removeItem('orca_commit_form')
  }
  void loadRecentMessages()
})
onBeforeRouteLeave((to) => {
  if (loading.value) return false
  if (to.name !== 'diff') sessionStorage.removeItem('orca_commit_form')
})

const openWorkspace = async () => {
  const success = await openWorkspaceDialog(t('dialog.selectSVNWorkspaceDirectory'))
  if (success) {
    router.push({ name: 'workspace' })
  }
}

const doCommit = async () => {
  if (!workspaceStore.currentPath || !commitMessage.value || selectedFiles.value.length === 0) {
    return
  }

  loading.value = true
  output.value = ''

  try {
    const availablePaths = new Set(allChangedFiles.value.map(file => file.path))
    const files = selectedFiles.value.filter(path => availablePaths.has(path))
    if (files.length === 0) return

    const targetFileSet = new Set(files)
    const unversionedFiles = allChangedFiles.value
      .filter(file => file.status_code === 'unversioned' && targetFileSet.has(file.path))
      .map(file => file.path)

    if (unversionedFiles.length > 0) {
      await svnAdd(workspaceStore.currentPath, unversionedFiles)
      await refreshStatus()
      selectedFiles.value = allChangedFiles.value
        .filter(file => targetFileSet.has(file.path) || unversionedFiles.some(parent => isWithinDirectory(file.path, parent)))
        .map(file => file.path)
      output.value = t('commit.reviewAdded')
      return
    }

    const result = await svnCommit(workspaceStore.currentPath, commitMessage.value, files)
    if (result.success) recordRecentMessage(workspaceStore.currentPath, commitMessage.value)
    output.value = result.output
    await refreshStatus()

    setTimeout(() => {
      router.push({ name: 'workspace' })
    }, 1500)
  } catch (err) {
    output.value = `${t('common.error')}：${err}`
  } finally {
    loading.value = false
  }
}

const viewDiff = (path: string) => {
  const allFiles = allChangedFiles.value.map(f => f.path)
  const index = allFiles.indexOf(path)

  sessionStorage.setItem('orca_commit_form', JSON.stringify({
    workspace: workspaceStore.currentPath,
    scope: scope.value,
    selectedFiles: selectedFiles.value,
    commitMessage: commitMessage.value,
  }))
  sessionStorage.setItem('orca_diff_files', JSON.stringify({
    source: 'commit',
    files: allFiles,
    current: path,
    index: Math.max(0, index),
  }))

  router.push({ name: 'diff', query: { path } })
}

const resetForm = () => {
  selectedFiles.value = []
  commitMessage.value = ''
  output.value = ''

}

const loadRecentMessages = () => {
  const workspacePath = workspaceStore.currentPath
  if (!workspacePath) return

  try {
    const stored = JSON.parse(localStorage.getItem(RECENT_COMMIT_MESSAGES_KEY) || '{}') as Record<string, unknown>
    const messages = stored[workspacePath]
    recentMessages.value = Array.isArray(messages)
      ? messages.filter((entry): entry is RecentCommitMessage => Boolean(entry && typeof entry === 'object' && typeof (entry as RecentCommitMessage).id === 'string' && typeof (entry as RecentCommitMessage).message === 'string'))
      : []
  } catch {
    recentMessages.value = []
  }
}

const recordRecentMessage = (workspacePath: string, message: string) => {
  const normalized = message.trim()
  if (!normalized) return

  try {
    const stored = JSON.parse(localStorage.getItem(RECENT_COMMIT_MESSAGES_KEY) || '{}') as Record<string, unknown>
    const previous = Array.isArray(stored[workspacePath])
      ? stored[workspacePath] as RecentCommitMessage[]
      : []
    stored[workspacePath] = [
      { id: `${Date.now()}-${Math.random().toString(36).slice(2)}`, message: normalized },
      ...previous.filter(entry => entry.message !== normalized),
    ]
    localStorage.setItem(RECENT_COMMIT_MESSAGES_KEY, JSON.stringify(stored))
    recentMessages.value = stored[workspacePath] as RecentCommitMessage[]
  } catch {
    // Ignore unavailable or malformed local storage.
  }
}

const useRecentMessage = (message: string) => {
  commitMessage.value = message
}
</script>

<style scoped>
.commit-scope { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; width: 100%; overflow-wrap: anywhere; margin-bottom: 12px; }
.recent-messages { margin-top: 10px; padding: 10px 12px; border: 1px solid var(--el-border-color-lighter); border-radius: var(--app-radius-md); background: var(--el-fill-color-lighter); }
.recent-messages-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-bottom: 6px; }
.recent-messages-title { display: inline-flex; align-items: center; gap: 6px; color: var(--el-text-color-regular); font-size: 12px; font-weight: 600; }
.recent-message-list { display: flex; flex-direction: column; gap: 4px; }
.recent-message-item { display: flex; align-items: center; gap: 8px; width: 100%; padding: 6px 8px; border: 0; border-radius: 4px; background: transparent; color: var(--el-text-color-primary); text-align: left; cursor: pointer; }
.recent-message-item:hover { background: var(--el-fill-color); }
.recent-message-revision { flex: 0 0 auto; color: var(--el-color-primary); font: 12px/1 "Cascadia Mono", Consolas, Monaco, monospace; }
.recent-message-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
.recent-messages-loading { color: var(--el-text-color-secondary); font-size: 12px; }
.commit-tree { width: 100%; max-height: 420px; overflow: auto; }
.commit-tree-row { display: flex; align-items: center; gap: 10px; flex: 1; min-width: 0; padding-right: 8px; }
.tree-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
.tree-count { color: var(--el-text-color-secondary); }
.commit-tree :deep(.el-tree-node__content) { height: 38px; }

.commit-view {
  max-width: 900px;
  margin: 0 auto;
  background: var(--md-sys-color-surface);
}

.commit-card {
  border-radius: var(--app-radius-lg);
  background: var(--md-sys-color-surface);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  display: inline-flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  font-weight: 700;
}

.no-workspace {
  padding: var(--app-spacing-xl) 0;
}

.commit-content {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-lg);
}

.commit-info {
  border-radius: var(--app-radius-md);
}

.info-content {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-sm);
  margin-top: var(--app-spacing-sm);
}

.info-item {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
}

.info-label {
  font-weight: 600;
  color: var(--el-text-color-regular);
}

.info-value {
  color: var(--el-text-color-primary);
}

.path-text {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
  word-break: break-all;
}

.commit-form {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-md);
}

.file-selection {
  margin-bottom: 0;
}

.file-table {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.file-path {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
}

.file-path-link {
  max-width: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--md-sys-color-primary);
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}

.file-path-link:hover {
  text-decoration: underline;
}

.message-input {
  position: relative;
}

.commit-textarea {
  font-family: inherit;
}

.char-count {
  position: absolute;
  right: 12px;
  bottom: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.char-count .warning {
  color: var(--md-sys-color-warning);
}

.form-actions {
  margin-bottom: 0;
}

.output-area {
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.output-header {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  padding: var(--app-spacing) var(--app-spacing-md);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
}

.output-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  font-size: 13px;
  color: var(--el-text-color-regular);
}

.output-textarea {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
}

:deep(.output-textarea .el-textarea__inner) {
  border: none;
  border-radius: 0;
  padding: var(--app-spacing-md);
}

.status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 82px;
  height: 28px;
  padding: 0 var(--app-spacing);
  border-radius: var(--app-radius-full);
  background: #f5f5fb;
  font-weight: 700;
  font-size: 12px;
}

.status-added {
  color: #15803d;
  background: #dcfce7;
}

.status-modified {
  color: #a16207;
  background: #fef9c3;
}

.status-deleted {
  color: #dc2626;
  background: #fee2e2;
}

.status-unversioned {
  color: #6366f1;
  background: #e0e7ff;
}

.commit-toolbar {
  display: flex;
  align-items: center;
  gap: var(--app-spacing);
}

.search-input {
  width: 240px;
}

.filter-tags {
  display: flex;
  gap: 6px;
}

@media (max-width: 640px) {
  .commit-view {
    padding: 0 var(--app-spacing);
  }
  
  .info-item {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
