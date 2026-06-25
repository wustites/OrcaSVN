<template>
  <div class="workspace-view">
    <!-- 未打开工作区时的空状态 -->
    <div v-if="!workspaceStore.currentPath" class="empty-state">
      <aside class="welcome-sidebar">
        <div class="sidebar-section">
          <div class="sidebar-heading">{{ $t('workspace.quickActions') }}</div>
          <button class="sidebar-row active" @click="openWorkspace">
            <el-icon><FolderOpened /></el-icon>
            {{ $t('workspace.openWorkspace') }}
          </button>
          <button class="sidebar-row" @click="doCheckout">
            <el-icon><Download /></el-icon>
            {{ $t('common.checkout') }}
          </button>
        </div>
      </aside>
      <div class="empty-content">
        <div class="welcome-mark"><el-icon><FolderOpened /></el-icon></div>
        <h2>Welcome to OrcaSVN</h2>
        <p>{{ $t('workspace.emptyDescription') }}</p>
        <div class="empty-actions">
          <el-button type="primary" @click="openWorkspace">
            <el-icon><FolderOpened /></el-icon>
            {{ $t('workspace.openWorkspace') }}
          </el-button>
          <el-button @click="doCheckout">
            <el-icon><Download /></el-icon>
            {{ $t('common.checkout') }}
          </el-button>
        </div>
      </div>
    </div>

    <!-- 工作区主界面 -->
    <div v-else class="workspace-layout">
      <!-- 左侧面板：工作区信息 -->
      <aside class="left-panel">
        <div class="repository-name">
          <strong>{{ workspaceName }}</strong>
          <button @click="closeWorkspace"><el-icon><Close /></el-icon></button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">WORKING COPY</div>
          <button class="sidebar-row active" @click="setFilter('all')">
            <el-icon><Document /></el-icon>
            <span>{{ $t('workspace.fileStatus') }}</span>
            <b>{{ workspaceStore.statusList.length }}</b>
          </button>
          <button class="sidebar-row" @click="doCommit">
            <el-icon><Upload /></el-icon>
            <span>{{ $t('common.commit') }}</span>
          </button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">REPOSITORY</div>
          <button class="sidebar-row" @click="router.push({ name: 'log' })">
            <el-icon><List /></el-icon>
            <span>{{ $t('menu.log') }}</span>
          </button>
          <button class="sidebar-row" @click="router.push({ name: 'blame' })">
            <el-icon><Edit /></el-icon>
            <span>{{ $t('menu.blame') }}</span>
          </button>
        </div>
        <div v-if="workspaceStore.svnInfo" class="repository-meta">
          <div><span>URL</span><strong>{{ workspaceStore.svnInfo.url }}</strong></div>
          <div><span>{{ $t('workspace.revision') }}</span><strong>r{{ workspaceStore.svnInfo.revision }}</strong></div>
        </div>
        <div class="sidebar-actions">
          <button @click="doCleanup"><el-icon><Brush /></el-icon>{{ $t('common.cleanup') }}</button>
        </div>
      </aside>

      <!-- 中间面板：文件列表 -->
      <main class="center-panel">
        <div class="panel-header">
          <span class="panel-title">
            <el-icon><List /></el-icon>
            {{ $t('workspace.fileStatus') }}
          </span>
          <div class="header-actions">
            <el-button text size="small" @click="refreshStatus" :loading="workspaceStore.isLoading">
              <el-icon><Refresh /></el-icon>
            </el-button>
          </div>
        </div>

        <!-- 文件状态统计 -->
        <div class="status-summary">
          <div class="status-badge modified" :class="{ active: filter === 'modified' }" @click="setFilter('modified')">
            <span class="badge-count">{{ workspaceStore.changedCount }}</span>
            <span class="badge-label">{{ $t('workspace.statusChanged') }}</span>
          </div>
          <div class="status-badge added" :class="{ active: filter === 'added' }" @click="setFilter('added')">
            <span class="badge-count">{{ workspaceStore.unversionedCount }}</span>
            <span class="badge-label">{{ $t('workspace.statusUnversioned') }}</span>
          </div>
          <div class="status-badge conflicted" :class="{ active: filter === 'conflicted' }" @click="setFilter('conflicted')">
            <span class="badge-count">{{ workspaceStore.conflictedCount }}</span>
            <span class="badge-label">{{ $t('status.conflicted') }}</span>
          </div>
          <div class="status-badge missing" :class="{ active: filter === 'missing' }" @click="setFilter('missing')">
            <span class="badge-count">{{ workspaceStore.missingCount }}</span>
            <span class="badge-label">{{ $t('status.missing') }}</span>
          </div>
          <div class="status-badge all" :class="{ active: filter === 'all' }" @click="setFilter('all')">
            <span class="badge-count">{{ workspaceStore.statusList.length }}</span>
            <span class="badge-label">{{ $t('common.all') }}</span>
          </div>
        </div>

        <!-- 文件列表 -->
        <div class="file-list">
          <div
            v-for="file in filteredFiles"
            :key="file.path"
            class="file-item"
            :class="{ selected: selectedFile === file.path }"
            @click="selectFile(file)"
          >
            <span class="file-status" :class="getStatusClass(file.status_code)">
              {{ getStatusLabel(file.status_code) }}
            </span>
            <span class="file-path" :title="file.path">{{ file.path }}</span>
            <div class="file-actions">
              <el-tooltip :content="$t('common.diff')" placement="top" :show-after="150">
                <span class="file-action-trigger" :title="$t('common.diff')" @click.stop>
                  <el-button text size="small" :aria-label="$t('common.diff')" @click="viewDiff(file.path)">
                    <el-icon><Connection /></el-icon>
                  </el-button>
                </span>
              </el-tooltip>
              <el-tooltip :content="$t('common.revert')" placement="top" :show-after="150">
                <span class="file-action-trigger" :title="$t('common.revert')" @click.stop>
                  <el-button text size="small" type="danger" :aria-label="$t('common.revert')" @click="revertFile(file)">
                    <el-icon><RefreshLeft /></el-icon>
                  </el-button>
                </span>
              </el-tooltip>
            </div>
          </div>
          <div v-if="filteredFiles.length === 0" class="empty-files">
            <el-icon><CircleCheck /></el-icon>
            <span>{{ $t('workspace.noChanges') }}</span>
          </div>
        </div>
      </main>

      <!-- 右侧面板：差异预览 -->
      <aside class="right-panel">
        <div class="panel-header">
          <span class="panel-title">
            <el-icon><Connection /></el-icon>
            {{ $t('diff.title') }}
          </span>
        </div>
        <div class="panel-content">
          <div v-if="!selectedFile" class="empty-diff">
            <el-icon><Document /></el-icon>
            <span>{{ $t('diff.selectFile') }}</span>
          </div>
          <div v-else-if="isLoadingDiff" class="loading-diff">
            <el-skeleton :rows="10" animated />
          </div>
          <div v-else-if="diffResult" class="diff-content">
            <div class="diff-header">
              <el-tag type="primary" size="small">{{ selectedFile }}</el-tag>
              <div class="diff-stats">
                <el-tag type="success" size="small">+{{ diffStats.added }}</el-tag>
                <el-tag type="danger" size="small">-{{ diffStats.removed }}</el-tag>
              </div>
            </div>
            <div class="diff-lines">
              <div
                v-for="line in diffLines"
                :key="line.index"
                class="diff-row"
                :class="line.className"
              >
                <span class="diff-line-number">{{ line.index }}</span>
                <span class="diff-marker">{{ line.marker }}</span>
                <code class="diff-code">{{ line.text }}</code>
              </div>
            </div>
          </div>
          <div v-else class="empty-diff">
            <el-icon><Document /></el-icon>
            <span>{{ $t('diff.noDiff') }}</span>
          </div>
        </div>
      </aside>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { deleteUnversioned, svnCleanup, svnRevert, svnDiff } from '@/api/svn'
import { useI18n } from 'vue-i18n'
import { getStatusClass, getStatusLabelKey } from '@/composables/useSvnStatus'
import { useWorkspace } from '@/composables/useWorkspace'
import type { SvnStatus, DiffResult } from '@/types'

const { t } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog, refreshStatus } = useWorkspace()

const filter = ref<'all' | 'modified' | 'added' | 'conflicted' | 'missing'>('all')
const selectedFile = ref<string | null>(null)
const isLoadingDiff = ref(false)
const diffResult = ref<DiffResult | null>(null)
const workspaceName = computed(() => {
  const path = workspaceStore.currentPath || ''
  return path.split(/[\\/]/).filter(Boolean).pop() || path
})

type DiffLineType = 'added' | 'removed' | 'context' | 'meta'

interface DiffLineRow {
  index: number
  marker: string
  text: string
  className: string
  type: DiffLineType
}

const filteredFiles = computed(() => {
  if (filter.value === 'all') return workspaceStore.statusList
  return workspaceStore.statusList.filter(f => {
    if (filter.value === 'modified') return ['modified', 'added', 'deleted', 'replaced'].includes(f.status_code)
    if (filter.value === 'added') return f.status_code === 'unversioned'
    if (filter.value === 'conflicted') return f.status_code === 'conflicted' || f.prop_status === 'conflicted'
    if (filter.value === 'missing') return f.status_code === 'missing'
    return true
  })
})

const diffLines = computed<DiffLineRow[]>(() => {
  if (!diffResult.value?.diff) return []
  const lines = diffResult.value.diff.split('\n')
  return lines.map((line, index) => {
    if (line.startsWith('+') && !line.startsWith('+++')) {
      return { index: index + 1, marker: '+', text: line.slice(1), className: 'diff-added', type: 'added' }
    }
    if (line.startsWith('-') && !line.startsWith('---')) {
      return { index: index + 1, marker: '-', text: line.slice(1), className: 'diff-removed', type: 'removed' }
    }
    const isMeta = line.startsWith('@@') || line.startsWith('+++') || line.startsWith('---')
    return { index: index + 1, marker: isMeta ? '@' : '', text: line, className: isMeta ? 'diff-meta' : 'diff-context', type: isMeta ? 'meta' : 'context' }
  })
})

const diffStats = computed(() => {
  return diffLines.value.reduce(
    (stats, line) => {
      if (line.type === 'added') stats.added += 1
      if (line.type === 'removed') stats.removed += 1
      return stats
    },
    { added: 0, removed: 0 }
  )
})

const setFilter = (f: typeof filter.value) => {
  filter.value = f
}

const selectFile = async (file: SvnStatus) => {
  selectedFile.value = file.path
  await loadDiff(file.path)
}

const loadDiff = async (path: string) => {
  if (!workspaceStore.currentPath) return
  isLoadingDiff.value = true
  diffResult.value = null
  try {
    diffResult.value = await svnDiff(workspaceStore.currentPath, path)
  } catch {
    diffResult.value = null
  } finally {
    isLoadingDiff.value = false
  }
}

const getStatusLabel = (code: string) => {
  return t(getStatusLabelKey(code))
}

const openWorkspace = () => openWorkspaceDialog(t('dialog.selectSVNWorkspaceDirectory'))

const closeWorkspace = () => {
  workspaceStore.clearWorkspace()
  selectedFile.value = null
  diffResult.value = null
}

const doCheckout = () => router.push({ name: 'checkout' })

const doCommit = () => router.push({ name: 'commit' })

const doCleanup = async () => {
  if (!workspaceStore.currentPath) return
  try {
    await svnCleanup(workspaceStore.currentPath)
    await refreshStatus()
  } catch (err) {
    workspaceStore.setError(String(err))
  }
}

const viewDiff = (path: string) => {
  router.push({ name: 'diff', query: { path } })
}

const revertFile = async (file: SvnStatus) => {
  if (!workspaceStore.currentPath) return
  const path = file.path
  try {
    if (file.status_code === 'unversioned') {
      await deleteUnversioned(workspaceStore.currentPath, [path])
    } else {
      await svnRevert(workspaceStore.currentPath, [path])
    }
    await refreshStatus()
    if (selectedFile.value === path) {
      selectedFile.value = null
      diffResult.value = null
    }
  } catch (err) {
    workspaceStore.setError(String(err))
  }
}
</script>

<style scoped>
.workspace-view {
  height: 100%;
}

/* 空状态 */
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 24px;
}

.empty-content {
  text-align: center;
  max-width: 480px;
}

.empty-icon {
  display: grid;
  place-items: center;
  width: 80px;
  height: 80px;
  margin: 0 auto 24px;
  border-radius: var(--app-radius-md);
  background: linear-gradient(135deg, var(--md-sys-color-primary-container), var(--md-sys-color-secondary-container));
  color: var(--md-sys-color-primary);
  font-size: 36px;
}

.empty-content h2 {
  margin: 0 0 8px;
  font-size: 24px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.empty-content p {
  margin: 0 0 24px;
  color: var(--el-text-color-secondary);
}

.empty-actions {
  display: flex;
  justify-content: center;
  gap: 12px;
}

/* 三栏布局 */
.workspace-layout {
  display: flex;
  height: 100%;
  overflow: hidden;
}

/* 面板通用样式 */
.left-panel,
.center-panel,
.right-panel {
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--md-sys-color-outline-variant);
}

.left-panel {
  width: 280px;
  flex-shrink: 0;
  background: var(--md-sys-color-surface);
}

.center-panel {
  flex: 1;
  min-width: 0;
  background: var(--md-sys-color-surface);
}

.right-panel {
  width: 400px;
  flex-shrink: 0;
  border-right: none;
  background: var(--md-sys-color-surface-dim);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 12px;
  background: var(--md-sys-color-surface-container);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  flex-shrink: 0;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.panel-title .el-icon {
  font-size: 14px;
}

.panel-content {
  flex: 1;
  overflow: auto;
  padding: 12px;
}

.panel-section {
  border-top: 1px solid var(--md-sys-color-outline-variant);
}

/* 左侧信息面板 */
.info-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.info-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.info-value {
  font-size: 13px;
  color: var(--el-text-color-primary);
  word-break: break-all;
}

.path-text {
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 12px;
}

.url-text {
  color: var(--md-sys-color-primary);
  font-size: 12px;
}

.no-info {
  text-align: center;
  color: var(--el-text-color-secondary);
  padding: 20px 0;
}

/* 快速操作 */
.quick-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
}

.quick-actions .el-button {
  justify-content: flex-start;
}

/* 状态统计 */
.status-summary {
  display: flex;
  padding: 8px 12px;
  gap: 8px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  flex-shrink: 0;
}

.status-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: var(--app-radius-full);
  background: var(--el-fill-color-light);
  cursor: pointer;
  transition: all var(--app-transition-fast);
  font-size: 12px;
}

.status-badge:hover {
  background: var(--el-fill-color);
}

.status-badge.active {
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-primary);
}

.badge-count {
  font-weight: 700;
}

.badge-label {
  color: var(--el-text-color-secondary);
}

.status-badge.modified .badge-count {
  color: var(--md-sys-color-warning);
}

.status-badge.added .badge-count {
  color: var(--md-sys-color-success);
}

.status-badge.deleted .badge-count {
  color: var(--md-sys-color-error);
}

.status-badge.conflicted .badge-count,
.status-badge.missing .badge-count {
  color: var(--md-sys-color-error);
}

/* 文件列表 */
.file-list {
  flex: 1;
  overflow: auto;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  cursor: pointer;
  transition: background var(--app-transition-fast);
}

.file-item:hover {
  background: var(--el-fill-color-lighter);
}

.file-item.selected {
  background: var(--md-sys-color-primary-container);
}

.file-status {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 24px;
  height: 20px;
  padding: 0 6px;
  border-radius: var(--app-radius-xs);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
}

.file-status.status-modified,
.file-status.status-replaced {
  background: #fef9c3;
  color: #a16207;
}

.file-status.status-added {
  background: #dcfce7;
  color: #15803d;
}

.file-status.status-deleted,
.file-status.status-missing,
.file-status.status-conflicted,
.file-status.status-obstructed {
  background: #fee2e2;
  color: #dc2626;
}

.file-status.status-unversioned {
  background: #e0e7ff;
  color: #6366f1;
}

.file-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
  font-family: "Cascadia Mono", Consolas, monospace;
}

.file-actions {
  display: flex;
  gap: 2px;
  opacity: .45;
  transition:
    opacity var(--app-transition-fast),
    transform var(--app-transition-fast);
}

.file-actions :deep(.el-button) {
  width: 26px;
  height: 24px;
  padding: 0;
  border: 1px solid transparent;
  background: var(--el-fill-color-light);
}

.file-actions :deep(.el-button:hover) {
  border-color: var(--md-sys-color-outline-variant);
  background: var(--el-fill-color);
}

.file-actions :deep(.el-button.el-button--danger:hover) {
  color: #fff;
  border-color: #bd3d45;
  background: #bd3d45;
}

.file-action-trigger {
  display: inline-flex;
}

.file-item:hover .file-actions {
  opacity: 1;
  transform: translateX(-1px);
}

.empty-files {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px;
  color: var(--el-text-color-secondary);
}

.empty-files .el-icon {
  font-size: 32px;
}

/* 差异预览 */
.empty-diff,
.loading-diff {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 100%;
  color: var(--el-text-color-secondary);
}

.empty-diff .el-icon {
  font-size: 48px;
}

.diff-content {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.diff-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  flex-shrink: 0;
}

.diff-stats {
  display: flex;
  gap: 6px;
}

.diff-lines {
  flex: 1;
  overflow: auto;
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
}

.diff-row {
  display: grid;
  grid-template-columns: 48px 24px 1fr;
  border-bottom: 1px solid rgba(226, 228, 238, 0.16);
}

.diff-line-number,
.diff-marker {
  user-select: none;
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-lighter);
  text-align: right;
}

.diff-line-number {
  padding: 2px 8px;
}

.diff-marker {
  padding: 2px 6px;
  text-align: center;
  font-weight: 700;
}

.diff-code {
  padding: 2px 12px;
  color: var(--el-text-color-primary);
  white-space: pre;
}

.diff-added {
  background: #ecfdf3;
}

.diff-added .diff-marker,
.diff-added .diff-line-number {
  background: #dcfce7;
  color: #15803d;
}

.diff-removed {
  background: #fff1f2;
}

.diff-removed .diff-marker,
.diff-removed .diff-line-number {
  background: #fee2e2;
  color: #dc2626;
}

.diff-meta {
  background: #eef2ff;
}

.diff-meta .diff-code,
.diff-meta .diff-marker {
  color: #4338ca;
  font-weight: 700;
}

/* 暗色主题 */
.theme-dark .diff-lines {
  background: #1a1a2e;
}

.theme-dark .diff-row {
  border-bottom-color: rgba(143, 160, 174, 0.1);
}

.theme-dark .diff-line-number,
.theme-dark .diff-marker {
  background: #2a2a3e;
  color: #8b8ba0;
}

.theme-dark .diff-code {
  color: #c4c4d8;
}

.theme-dark .diff-added {
  background: #052e16;
}

.theme-dark .diff-added .diff-marker,
.theme-dark .diff-added .diff-line-number {
  background: #052e16;
  color: #4ade80;
}

.theme-dark .diff-removed {
  background: #450a0a;
}

.theme-dark .diff-removed .diff-marker,
.theme-dark .diff-removed .diff-line-number {
  background: #450a0a;
  color: #f87171;
}

.theme-dark .diff-meta {
  background: #1e1b4b;
}

.theme-dark .diff-meta .diff-code,
.theme-dark .diff-meta .diff-marker {
  color: #a5b4fc;
}

/* 响应式 */
@media (max-width: 1200px) {
  .right-panel {
    width: 320px;
  }
}

@media (max-width: 860px) {
  .left-panel {
    display: none;
  }
  
  .right-panel {
    display: none;
  }
}
</style>

<style scoped>
/* Fork-like workspace layout overrides */
.empty-state {
  display: grid;
  grid-template-columns: 210px 1fr;
  align-items: stretch;
  justify-content: stretch;
  padding: 0;
  background: #fff;
}

.welcome-sidebar,
.left-panel {
  width: 210px;
  background: linear-gradient(90deg, #eeeeee, #e8e8e8);
  border-right: 1px solid #c8c8c8;
  color: #444;
}

.empty-content {
  align-self: start;
  max-width: 680px;
  margin: 42px 44px;
  text-align: left;
}

.welcome-mark {
  display: grid;
  place-items: center;
  width: 54px;
  height: 54px;
  margin-bottom: 20px;
  border-radius: 11px;
  color: #fff;
  background: linear-gradient(145deg, #42a5ff, #0569d7);
  font-size: 28px;
  box-shadow: 0 2px 5px rgba(0, 0, 0, 0.18);
}

.empty-content h2 {
  margin-bottom: 8px;
  font-size: 24px;
  font-weight: 500;
}

.empty-content p {
  margin-bottom: 20px;
  color: #777;
}

.empty-actions {
  justify-content: flex-start;
}

.workspace-layout {
  background: #fff;
}

.left-panel {
  flex-shrink: 0;
}

.repository-name {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 37px;
  padding: 0 9px 0 12px;
  border-bottom: 1px solid #c8c8c8;
  color: #333;
  font-size: 12px;
}

.repository-name button {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border: 0;
  border-radius: 3px;
  background: transparent;
  color: #777;
}

.repository-name button:hover,
.sidebar-row:hover {
  background: rgba(255, 255, 255, 0.65);
}

.sidebar-section {
  padding: 8px 6px 3px;
}

.sidebar-heading {
  padding: 3px 7px;
  color: #707070;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: .06em;
}

.sidebar-row {
  display: flex;
  align-items: center;
  width: 100%;
  height: 25px;
  gap: 6px;
  padding: 0 8px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #444;
  font-size: 11px;
  text-align: left;
}

.sidebar-row.active {
  color: #fff;
  background: #1473e6;
}

.sidebar-row span {
  flex: 1;
}

.sidebar-row b {
  font-size: 10px;
}

.repository-meta {
  margin: 10px 12px;
  padding-top: 8px;
  border-top: 1px solid #c9c9c9;
}

.repository-meta div {
  margin-bottom: 9px;
}

.repository-meta span,
.repository-meta strong {
  display: block;
}

.repository-meta span {
  color: #888;
  font-size: 9px;
  text-transform: uppercase;
}

.repository-meta strong {
  overflow: hidden;
  margin-top: 2px;
  color: #555;
  font-size: 10px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar-actions {
  display: grid;
  gap: 4px;
  margin-top: auto;
  padding: 9px;
  border-top: 1px solid #c8c8c8;
}

.sidebar-actions button {
  display: flex;
  align-items: center;
  height: 27px;
  gap: 6px;
  padding: 0 8px;
  border: 1px solid #c3c3c3;
  border-radius: 4px;
  background: linear-gradient(#fff, #e9e9e9);
  color: #444;
  font-size: 10px;
}

.center-panel,
.right-panel {
  background: #fff;
}

.center-panel {
  flex: 1 1 33.333%;
  width: 33.333%;
  min-width: 260px;
}

.right-panel {
  flex: 2 1 66.667%;
  width: 66.667%;
  min-width: 420px;
  background: #fff;
  border-left: 1px solid #c8c8c8;
}

.panel-header {
  height: 37px;
  padding: 0 9px;
  background: linear-gradient(#f7f7f7, #e9e9e9);
  border-bottom: 1px solid #c8c8c8;
}

.panel-title {
  color: #444;
  font-size: 11px;
  font-weight: 600;
}

.status-summary {
  gap: 3px;
  padding: 5px 7px;
  background: #f4f4f4;
  border-bottom: 1px solid #d5d5d5;
}

.status-badge {
  gap: 5px;
  padding: 2px 7px;
  border: 1px solid transparent;
  border-radius: 4px;
  background: transparent;
  font-size: 10px;
}

.status-badge.active {
  border-color: #b9b9b9;
  background: #ddd;
  color: #222;
}

.file-item {
  min-height: 27px;
  gap: 7px;
  padding: 3px 8px;
  border-bottom: 1px solid #ededed;
}

.file-item.selected {
  color: #fff;
  background: #1473e6;
}

.file-status {
  min-width: 18px;
  height: 17px;
  padding: 0 3px;
  font-size: 8px;
}

.file-path {
  font-size: 11px;
}

.diff-header {
  min-height: 31px;
  padding: 4px 8px;
  background: #f7f7f7;
}

.diff-lines {
  font-size: 10px;
  line-height: 1.45;
}

.diff-row {
  grid-template-columns: 38px 18px 1fr;
}

.diff-line-number,
.diff-marker,
.diff-code {
  padding-top: 1px;
  padding-bottom: 1px;
}

@media (max-width: 860px) {
  .empty-state {
    grid-template-columns: 1fr;
  }
  .welcome-sidebar {
    display: none;
  }
}
</style>
