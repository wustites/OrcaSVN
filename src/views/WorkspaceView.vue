<template>
  <div class="workspace-view">
    <!-- 未打开工作区时的空状态 -->
    <div v-if="!workspaceStore.currentPath" class="empty-state">
      <div class="empty-content">
        <div class="empty-icon">
          <el-icon><FolderOpened /></el-icon>
        </div>
        <h2>{{ $t('workspace.emptyWorkspace') }}</h2>
        <p>{{ $t('workspace.emptyDescription') }}</p>
        <div class="empty-actions">
          <el-button type="primary" @click="openWorkspace" size="large">
            <el-icon><FolderOpened /></el-icon>
            {{ $t('workspace.openWorkspace') }}
          </el-button>
          <el-button @click="doCheckout" size="large">
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
        <div class="panel-header">
          <span class="panel-title">
            <el-icon><InfoFilled /></el-icon>
            {{ $t('workspace.repositoryInfo') }}
          </span>
          <el-button text size="small" @click="closeWorkspace">
            <el-icon><Close /></el-icon>
          </el-button>
        </div>
        <div class="panel-content">
          <div v-if="workspaceStore.svnInfo" class="info-list">
            <div class="info-item">
              <span class="info-label">{{ $t('workspace.path') }}</span>
              <span class="info-value path-text">{{ workspaceStore.svnInfo.path }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">URL</span>
              <span class="info-value url-text">{{ workspaceStore.svnInfo.url }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">{{ $t('workspace.repositoryRoot') }}</span>
              <span class="info-value path-text">{{ workspaceStore.svnInfo.repository_root }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">{{ $t('workspace.revision') }}</span>
              <el-tag type="primary" size="small">r{{ workspaceStore.svnInfo.revision }}</el-tag>
            </div>
          </div>
          <div v-else-if="!workspaceStore.isLoading" class="no-info">
            <span>{{ $t('workspace.noRepositoryInfo') }}</span>
          </div>
        </div>

        <!-- 快速操作 -->
        <div class="panel-section">
          <div class="panel-header">
            <span class="panel-title">
              <el-icon><Operation /></el-icon>
              {{ $t('workspace.quickActions') }}
            </span>
          </div>
          <div class="quick-actions">
            <el-button @click="doUpdate" :loading="isUpdating" size="small">
              <el-icon><RefreshRight /></el-icon>
              {{ $t('common.update') }}
            </el-button>
            <el-button @click="doCommit" size="small">
              <el-icon><Upload /></el-icon>
              {{ $t('common.commit') }}
            </el-button>
            <el-button @click="doCleanup" size="small">
              <el-icon><Brush /></el-icon>
              {{ $t('common.cleanup') }}
            </el-button>
          </div>
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
            <span class="badge-count">{{ workspaceStore.modifiedCount }}</span>
            <span class="badge-label">{{ $t('workspace.statusModified') }}</span>
          </div>
          <div class="status-badge added" :class="{ active: filter === 'added' }" @click="setFilter('added')">
            <span class="badge-count">{{ workspaceStore.addedCount }}</span>
            <span class="badge-label">{{ $t('workspace.statusAdded') }}</span>
          </div>
          <div class="status-badge deleted" :class="{ active: filter === 'deleted' }" @click="setFilter('deleted')">
            <span class="badge-count">{{ workspaceStore.deletedCount }}</span>
            <span class="badge-label">{{ $t('workspace.statusDeleted') }}</span>
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
              <el-tooltip :content="$t('common.diff')" placement="top">
                <el-button text size="small" @click.stop="viewDiff(file.path)">
                  <el-icon><Connection /></el-icon>
                </el-button>
              </el-tooltip>
              <el-tooltip :content="$t('common.revert')" placement="top">
                <el-button text size="small" type="danger" @click.stop="revertFile(file.path)">
                  <el-icon><RefreshLeft /></el-icon>
                </el-button>
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
import { ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnUpdate, svnCleanup, svnRevert, svnDiff } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'
import { getStatusClass, getStatusLabelKey } from '@/composables/useSvnStatus'
import { useWorkspace } from '@/composables/useWorkspace'
import type { SvnStatus, DiffResult } from '@/types'

const { t } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog, refreshStatus } = useWorkspace()

const filter = ref<'all' | 'modified' | 'added' | 'deleted'>('all')
const selectedFile = ref<string | null>(null)
const isUpdating = ref(false)
const isLoadingDiff = ref(false)
const diffResult = ref<DiffResult | null>(null)

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
    if (filter.value === 'modified') return f.status_code === 'modified'
    if (filter.value === 'added') return f.status_code === 'added' || f.status_code === 'unversioned'
    if (filter.value === 'deleted') return f.status_code === 'deleted'
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

const doUpdate = async () => {
  if (!workspaceStore.currentPath) return
  isUpdating.value = true
  try {
    await svnUpdate(workspaceStore.currentPath)
    await refreshStatus()
  } catch (err) {
    workspaceStore.error = String(err)
  } finally {
    isUpdating.value = false
  }
}

const doCleanup = async () => {
  if (!workspaceStore.currentPath) return
  try {
    await svnCleanup(workspaceStore.currentPath)
    await refreshStatus()
  } catch (err) {
    workspaceStore.error = String(err)
  }
}

const viewDiff = (path: string) => {
  router.push({ name: 'diff', query: { path } })
}

const revertFile = async (path: string) => {
  if (!workspaceStore.currentPath) return
  try {
    await svnRevert(workspaceStore.currentPath, [path])
    await refreshStatus()
    if (selectedFile.value === path) {
      selectedFile.value = null
      diffResult.value = null
    }
  } catch (err) {
    workspaceStore.error = String(err)
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

.file-status.modified {
  background: #fef9c3;
  color: #a16207;
}

.file-status.added {
  background: #dcfce7;
  color: #15803d;
}

.file-status.deleted {
  background: #fee2e2;
  color: #dc2626;
}

.file-status.unversioned {
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
  opacity: 0;
  transition: opacity var(--app-transition-fast);
}

.file-item:hover .file-actions {
  opacity: 1;
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
  border-bottom: 1px solid rgba(226, 228, 238, 0.3);
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
