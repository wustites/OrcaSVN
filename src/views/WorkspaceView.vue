<template>
  <div class="workspace-view">
    <div v-if="!workspaceStore.currentPath" class="empty-workspace">
      <div class="empty-panel animate-scale-in">
        <div class="empty-mark">
          <el-icon><FolderOpened /></el-icon>
        </div>
        <h1>{{ $t('workspace.emptyWorkspace') }}</h1>
        <p class="empty-description">{{ $t('workspace.emptyDescription') }}</p>
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

    <div v-else class="workspace-content">
      <el-card class="info-card animate-fade-in">
        <template #header>
          <div class="card-header">
            <span class="card-title">
              <el-icon><InfoFilled /></el-icon>
              {{ $t('workspace.repositoryInfo') }}
            </span>
            <el-button @click="closeWorkspace" size="small" text>
              <el-icon><Close /></el-icon>
              {{ $t('common.close') }}
            </el-button>
          </div>
        </template>
        <el-alert v-if="workspaceStore.error" type="error" :title="workspaceStore.error" :closable="false" class="mb-4" show-icon />
        <el-descriptions :column="responsiveColumns" border v-if="workspaceStore.svnInfo" class="info-descriptions">
          <el-descriptions-item :label="$t('workspace.path')">
            <span class="path-text">{{ workspaceStore.svnInfo.path }}</span>
          </el-descriptions-item>
          <el-descriptions-item label="URL">
            <span class="url-text">{{ workspaceStore.svnInfo.url }}</span>
          </el-descriptions-item>
          <el-descriptions-item :label="$t('workspace.repositoryRoot')">
            <span class="path-text">{{ workspaceStore.svnInfo.repository_root }}</span>
          </el-descriptions-item>
          <el-descriptions-item :label="$t('workspace.revision')">
            <el-tag type="primary" size="small">r{{ workspaceStore.svnInfo.revision }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item :label="$t('workspace.type')">
            <el-tag size="small">{{ workspaceStore.svnInfo.node_kind }}</el-tag>
          </el-descriptions-item>
        </el-descriptions>
        <div v-else-if="!workspaceStore.isLoading" class="no-info">
          <el-icon><InfoFilled /></el-icon>
          <span>{{ $t('workspace.noRepositoryInfo') }}</span>
        </div>
      </el-card>

      <el-card class="status-card animate-fade-in" style="animation-delay: 0.1s">
        <template #header>
          <div class="card-header">
            <span class="card-title">
              <el-icon><List /></el-icon>
              {{ $t('workspace.fileStatus') }}
            </span>
            <div class="header-actions">
              <el-button @click="refreshStatus" :loading="workspaceStore.isLoading" size="small">
                <el-icon><Refresh /></el-icon>
                {{ $t('common.refresh') }}
              </el-button>
              <el-button type="primary" @click="doUpdate" size="small">
                <el-icon><RefreshRight /></el-icon>
                {{ $t('common.update') }}
              </el-button>
            </div>
          </div>
        </template>

        <div class="status-summary">
          <div class="status-metric modified">
            <div class="metric-icon">
              <el-icon><Edit /></el-icon>
            </div>
            <div class="metric-info">
              <span class="metric-label">{{ $t('workspace.statusModified') }}</span>
              <strong class="metric-value">{{ workspaceStore.modifiedCount }}</strong>
            </div>
          </div>
          <div class="status-metric added">
            <div class="metric-icon">
              <el-icon><Plus /></el-icon>
            </div>
            <div class="metric-info">
              <span class="metric-label">{{ $t('workspace.statusAdded') }}</span>
              <strong class="metric-value">{{ workspaceStore.addedCount }}</strong>
            </div>
          </div>
          <div class="status-metric deleted">
            <div class="metric-icon">
              <el-icon><Delete /></el-icon>
            </div>
            <div class="metric-info">
              <span class="metric-label">{{ $t('workspace.statusDeleted') }}</span>
              <strong class="metric-value">{{ workspaceStore.deletedCount }}</strong>
            </div>
          </div>
        </div>

        <el-table 
          :data="workspaceStore.statusList" 
          style="width: 100%" 
          max-height="400"
          stripe
          highlight-current-row
          class="status-table"
        >
          <el-table-column prop="status_code" :label="$t('commit.status')" width="140" align="center">
            <template #default="{ row }">
              <span class="status-badge" :class="getStatusClass(row.status_code)">
                {{ $t(getStatusLabelKey(row.status_code)) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column prop="path" :label="$t('commit.file')" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="file-path">{{ row.path }}</span>
            </template>
          </el-table-column>
          <el-table-column :label="$t('common.action')" width="200" fixed="right" align="center">
            <template #default="{ row }">
              <div class="row-actions">
                <el-button link type="primary" @click="viewDiff(row.path)" size="small">
                  <el-icon><Connection /></el-icon>
                  {{ $t('common.diff') }}
                </el-button>
                <el-button link type="primary" @click="viewBlame(row.path)" size="small">
                  <el-icon><Edit /></el-icon>
                  {{ $t('common.blame') }}
                </el-button>
                <el-button link type="danger" @click="revertFile(row.path)" size="small">
                  <el-icon><RefreshLeft /></el-icon>
                  {{ $t('common.revert') }}
                </el-button>
              </div>
            </template>
          </el-table-column>
        </el-table>
      </el-card>

      <el-card class="actions-card animate-fade-in" style="animation-delay: 0.2s">
        <template #header>
          <span class="card-title">
            <el-icon><Operation /></el-icon>
            {{ $t('workspace.quickActions') }}
          </span>
        </template>
        <div class="quick-actions">
          <el-button @click="doCommit" class="action-btn">
            <el-icon><Upload /></el-icon>
            {{ $t('common.commit') }}
          </el-button>
          <el-button @click="doAdd" class="action-btn">
            <el-icon><Plus /></el-icon>
            {{ $t('common.add') }}
          </el-button>
          <el-button @click="doDelete" class="action-btn">
            <el-icon><Delete /></el-icon>
            {{ $t('common.delete') }}
          </el-button>
          <el-button @click="doCleanup" class="action-btn">
            <el-icon><Brush /></el-icon>
            {{ $t('common.cleanup') }}
          </el-button>
          <el-button @click="doSwitch" class="action-btn">
            <el-icon><Switch /></el-icon>
            {{ $t('common.switch') }}
          </el-button>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnUpdate, svnCleanup, svnRevert, svnAdd, svnDelete, svnSwitch } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessageBox } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { getStatusClass, getStatusLabelKey } from '@/composables/useSvnStatus'
import { useWorkspace } from '@/composables/useWorkspace'

const { t } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog, refreshStatus } = useWorkspace()

const responsiveColumns = computed(() => {
  return window.innerWidth > 860 ? 2 : 1
})

const openWorkspace = () => openWorkspaceDialog(t('dialog.selectSVNWorkspaceDirectory'))

const doCheckout = () => {
  router.push({ name: 'checkout' })
}

const closeWorkspace = () => {
  workspaceStore.clearWorkspace()
}

const doUpdate = async () => {
  if (!workspaceStore.currentPath) return

  try {
    await svnUpdate(workspaceStore.currentPath)
    await refreshStatus()
  } catch (err) {
    workspaceStore.error = String(err)
  }
}

const doCommit = () => {
  router.push({ name: 'commit' })
}

const doAdd = async () => {
  if (!workspaceStore.currentPath) return

  const selected = await open({
    multiple: true,
    title: t('common.add'),
  })

  if (!selected) return

  const files = Array.isArray(selected) ? selected : [selected]
  try {
    await svnAdd(workspaceStore.currentPath, files)
    await refreshStatus()
  } catch (err) {
    workspaceStore.error = String(err)
  }
}

const doDelete = async () => {
  if (!workspaceStore.currentPath) return

  const selected = await open({
    multiple: true,
    title: t('common.delete'),
  })

  if (!selected) return

  const files = Array.isArray(selected) ? selected : [selected]
  try {
    await svnDelete(workspaceStore.currentPath, files)
    await refreshStatus()
  } catch (err) {
    workspaceStore.error = String(err)
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

const doSwitch = async () => {
  if (!workspaceStore.currentPath) return

  try {
    const { value } = await ElMessageBox.prompt(
      t('switch.enterUrl'),
      t('common.switch'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        inputPattern: /^https?:\/\/.+/,
        inputErrorMessage: t('switch.invalidUrl'),
      }
    )

    if (value) {
      await svnSwitch(workspaceStore.currentPath, value)
      await refreshStatus()
    }
  } catch {
    // user cancelled
  }
}

const viewDiff = (path: string) => {
  router.push({ name: 'diff', query: { path } })
}

const viewBlame = (path: string) => {
  router.push({ name: 'blame', query: { path } })
}

const revertFile = async (path: string) => {
  if (!workspaceStore.currentPath) return

  try {
    await svnRevert(workspaceStore.currentPath, [path])
    await refreshStatus()
  } catch (err) {
    workspaceStore.error = String(err)
  }
}
</script>

<style scoped>
.workspace-view {
  height: 100%;
}

.empty-workspace {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  padding: var(--app-spacing-lg);
}

.empty-panel {
  width: min(520px, 100%);
  padding: var(--app-spacing-xl);
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-lg);
  background: rgba(255, 255, 255, 0.82);
  box-shadow: var(--md-sys-elevation-2);
  text-align: center;
  backdrop-filter: blur(20px);
}

.empty-panel h1 {
  margin: var(--app-spacing-lg) 0 var(--app-spacing-sm);
  color: #20212a;
  font-size: 24px;
  font-weight: 800;
  letter-spacing: -0.02em;
  line-height: 1.3;
}

.empty-description {
  color: var(--el-text-color-secondary);
  margin-bottom: var(--app-spacing-lg);
  font-size: 14px;
}

.empty-mark {
  display: grid;
  place-items: center;
  width: 80px;
  height: 80px;
  margin: 0 auto;
  border-radius: var(--app-radius-md);
  background: linear-gradient(135deg, var(--md-sys-color-primary-container), var(--md-sys-color-secondary-container));
  color: var(--md-sys-color-primary);
  font-size: 36px;
  box-shadow: var(--md-sys-elevation-2);
}

.empty-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--app-spacing);
}

.workspace-content {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: var(--app-spacing-md);
  padding: 2px;
  height: 100%;
  overflow-y: auto;
}

.info-card,
.status-card,
.actions-card {
  flex-shrink: 0;
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

.header-actions {
  display: flex;
  gap: var(--app-spacing-sm);
}

.info-descriptions {
  margin-top: var(--app-spacing-sm);
}

.path-text {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
  word-break: break-all;
}

.url-text {
  color: var(--md-sys-color-primary);
  word-break: break-all;
}

.no-info {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--app-spacing-sm);
  color: var(--el-text-color-secondary);
  text-align: center;
  padding: var(--app-spacing-lg);
}

.status-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--app-spacing);
  margin-bottom: var(--app-spacing-md);
}

.status-metric {
  display: flex;
  align-items: center;
  gap: var(--app-spacing);
  min-height: 80px;
  padding: var(--app-spacing-md);
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
  background: #fff;
  transition: transform var(--app-transition-fast), box-shadow var(--app-transition-fast);
}

.status-metric:hover {
  transform: translateY(-2px);
  box-shadow: var(--md-sys-elevation-2);
}

.metric-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: var(--app-radius);
  font-size: 20px;
  flex-shrink: 0;
}

.status-metric.modified .metric-icon {
  background: #fef9c3;
  color: #a16207;
}

.status-metric.added .metric-icon {
  background: #dcfce7;
  color: #15803d;
}

.status-metric.deleted .metric-icon {
  background: #fee2e2;
  color: #dc2626;
}

.metric-info {
  overflow: hidden;
}

.metric-label {
  display: block;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.metric-value {
  display: block;
  margin-top: 4px;
  color: #20212a;
  font-size: 28px;
  font-weight: 800;
  line-height: 1;
}

.status-metric.modified {
  background: linear-gradient(135deg, #fef9c3, #fff);
}

.status-metric.added {
  background: linear-gradient(135deg, #dcfce7, #fff);
}

.status-metric.deleted {
  background: linear-gradient(135deg, #fee2e2, #fff);
}

.status-table {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.file-path {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
}

.quick-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: var(--app-spacing);
}

.action-btn {
  justify-content: flex-start;
}

.row-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 82px;
  height: 28px;
  padding: 0 var(--app-spacing);
  border-radius: var(--app-radius-full);
  background: #f5f5fb;
  font-weight: 700;
  font-size: 12px;
}

.mb-4 {
  margin-bottom: var(--app-spacing-md);
}

@media (max-width: 860px) {
  .status-summary {
    grid-template-columns: 1fr;
  }
  
  .quick-actions {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .empty-panel {
    padding: var(--app-spacing-lg);
  }
  
  .empty-panel h1 {
    font-size: 20px;
  }
  
  .status-metric {
    min-height: 70px;
    padding: var(--app-spacing);
  }
  
  .metric-value {
    font-size: 24px;
  }
  
  .quick-actions {
    grid-template-columns: 1fr;
  }
}
</style>
