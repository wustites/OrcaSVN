<template>
  <div class="log-view">
    <el-card class="log-card">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon><Document /></el-icon>
            {{ $t('log.title') }}
          </span>
          <div class="header-actions">
            <el-input
              v-model.number="limit"
              type="number"
              :min="1"
              :max="1000"
              style="width: 120px"
              :placeholder="$t('log.quantity')"
              size="small"
            />
            <el-button @click="loadLogs" :loading="loading" type="primary" size="small">
              <el-icon><Refresh /></el-icon>
              {{ $t('log.load') }}
            </el-button>
          </div>
        </div>
      </template>

      <div v-if="!workspaceStore.currentPath" class="no-workspace">
        <el-empty :description="$t('log.openWorkspaceFirst')">
          <template #image>
            <el-icon class="empty-icon"><Document /></el-icon>
          </template>
          <el-button type="primary" @click="openWorkspace">
            <el-icon><FolderOpened /></el-icon>
            {{ $t('common.open') }}
          </el-button>
        </el-empty>
      </div>

      <div v-else class="log-content">
        <el-table
          :data="logs"
          style="width: 100%"
          max-height="600"
          @row-click="handleRowClick"
          row-key="revision"
          stripe
          highlight-current-row
          class="log-table"
        >
          <el-table-column prop="revision" :label="$t('log.revision')" width="90" sortable align="center">
            <template #default="{ row }">
              <el-tag type="primary" size="small" effect="plain">r{{ row.revision }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="author" :label="$t('log.author')" width="120">
            <template #default="{ row }">
              <div class="author-cell">
                <el-icon><User /></el-icon>
                <span>{{ row.author }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="date" :label="$t('log.date')" width="180" sortable>
            <template #default="{ row }">
              <div class="date-cell">
                <el-icon><Calendar /></el-icon>
                <span>{{ formatDate(row.date) }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="message" :label="$t('log.commitMessage')" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="message-text">{{ row.message }}</span>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <el-dialog
        v-model="dialogVisible"
        :title="$t('log.commitDetails')"
        width="60%"
        class="log-dialog"
        destroy-on-close
      >
        <div v-if="selectedLog" class="log-detail">
          <el-descriptions :column="2" border class="detail-descriptions">
            <el-descriptions-item :label="$t('log.revision')">
              <el-tag type="primary" size="small">r{{ selectedLog.revision }}</el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('log.author')">
              <div class="author-info">
                <el-icon><User /></el-icon>
                <span>{{ selectedLog.author }}</span>
              </div>
            </el-descriptions-item>
            <el-descriptions-item :label="$t('log.date')">
              <div class="date-info">
                <el-icon><Calendar /></el-icon>
                <span>{{ formatDate(selectedLog.date) }}</span>
              </div>
            </el-descriptions-item>
          </el-descriptions>

          <div class="commit-message-section">
            <h4 class="message-title">
              <el-icon><ChatLineSquare /></el-icon>
              {{ $t('log.commitMessage') }}
            </h4>
            <div class="commit-message">
              <pre>{{ selectedLog.message }}</pre>
            </div>
          </div>
        </div>
        
        <template #footer>
          <el-button @click="dialogVisible = false">
            {{ $t('common.close') }}
          </el-button>
        </template>
      </el-dialog>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnLog } from '@/api/svn'
import type { SvnLogEntry } from '@/types'
import { useI18n } from 'vue-i18n'
import { useWorkspace } from '@/composables/useWorkspace'

const { t, locale } = useI18n()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog } = useWorkspace()

const limit = ref(50)
const logs = ref<SvnLogEntry[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const selectedLog = ref<SvnLogEntry | null>(null)

const openWorkspace = async () => {
  const success = await openWorkspaceDialog(t('dialog.selectSVNWorkspaceDirectory'))
  if (success) {
    loadLogs()
  }
}

const loadLogs = async () => {
  if (!workspaceStore.currentPath) return

  loading.value = true
  try {
    logs.value = await svnLog(workspaceStore.currentPath, limit.value)
  } catch (err) {
    workspaceStore.error = String(err)
  } finally {
    loading.value = false
  }
}

const handleRowClick = (row: SvnLogEntry) => {
  selectedLog.value = row
  dialogVisible.value = true
}

const formatDate = (dateStr: string): string => {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  return date.toLocaleString(locale.value)
}

onMounted(() => {
  if (workspaceStore.currentPath) {
    loadLogs()
  }
})
</script>

<style scoped>
.log-view {
  height: 100%;
}

.log-card {
  height: 100%;
  border-radius: var(--app-radius-lg);
}

:deep(.log-card > .el-card__body) {
  display: flex;
  flex-direction: column;
  height: calc(100% - 57px);
  min-height: 0;
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

.no-workspace {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--app-spacing-xl) 0;
}

.empty-icon {
  font-size: 64px;
  color: var(--el-text-color-placeholder);
}

.log-content {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.log-table {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.author-cell,
.date-cell {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--el-text-color-regular);
}

.author-cell .el-icon,
.date-cell .el-icon {
  font-size: 14px;
  color: var(--el-text-color-secondary);
}

.message-text {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-dialog {
  border-radius: var(--app-radius-lg);
}

.log-detail {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-lg);
}

.detail-descriptions {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.author-info,
.date-info {
  display: flex;
  align-items: center;
  gap: 6px;
}

.author-info .el-icon,
.date-info .el-icon {
  font-size: 14px;
  color: var(--el-text-color-secondary);
}

.commit-message-section {
  margin-top: var(--app-spacing-sm);
}

.message-title {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  margin-bottom: var(--app-spacing);
  font-size: 15px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.message-title .el-icon {
  font-size: 18px;
  color: var(--md-sys-color-primary);
}

.commit-message {
  background: var(--el-fill-color-light);
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
  padding: var(--app-spacing-md);
  overflow: auto;
}

.commit-message pre {
  margin: 0;
  font-family: inherit;
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--el-text-color-primary);
}

@media (max-width: 860px) {
  .card-header {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--app-spacing-sm);
  }
  
  .header-actions {
    width: 100%;
  }
}

@media (max-width: 640px) {
  .log-dialog {
    width: 90% !important;
  }
}
</style>
