<template>
  <div class="log-view">
    <el-card class="log-card">
      <template #header>
        <div class="card-header">
          <span>{{ $t('log.title') }}</span>
          <div class="header-actions">
            <el-input
              v-model.number="limit"
              type="number"
              :min="1"
              :max="1000"
              style="width: 120px"
              :placeholder="$t('log.quantity')"
            />
            <el-button @click="loadLogs" :loading="loading">
              <el-icon><Refresh /></el-icon>
              {{ $t('common.load') }}
            </el-button>
          </div>
        </div>
      </template>

      <div v-if="!workspaceStore.currentPath" class="no-workspace">
        <el-empty :description="$t('log.openWorkspaceFirst')">
          <el-button type="primary" @click="openWorkspace">{{ $t('common.open') }}</el-button>
        </el-empty>
      </div>

      <div v-else>
        <el-table
          :data="logs"
          style="width: 100%"
          max-height="600"
          @row-click="handleRowClick"
          row-key="revision"
        >
          <el-table-column prop="revision" :label="$t('log.revision')" width="80" sortable />
          <el-table-column prop="author" :label="$t('log.author')" width="120" />
          <el-table-column prop="date" :label="$t('log.date')" width="180" sortable>
            <template #default="{ row }">
              {{ formatDate(row.date) }}
            </template>
          </el-table-column>
          <el-table-column prop="message" :label="$t('log.commitMessage')">
            <template #default="{ row }">
              <el-text truncated :title="row.message">{{ row.message }}</el-text>
            </template>
          </el-table-column>
        </el-table>

        <el-dialog
          v-model="dialogVisible"
          :title="$t('log.commitDetails')"
          width="60%"
        >
          <div v-if="selectedLog" class="log-detail">
            <el-descriptions :column="2" border>
              <el-descriptions-item :label="$t('log.revision')">{{ selectedLog.revision }}</el-descriptions-item>
              <el-descriptions-item :label="$t('log.author')">{{ selectedLog.author }}</el-descriptions-item>
              <el-descriptions-item :label="$t('log.date')">{{ formatDate(selectedLog.date) }}</el-descriptions-item>
            </el-descriptions>

            <h4>{{ $t('log.commitMessage') }}</h4>
            <p class="commit-message">{{ selectedLog.message }}</p>
          </div>
        </el-dialog>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnLog } from '@/api/svn'
import type { SvnLogEntry } from '@/types'
import { open } from '@tauri-apps/plugin-dialog'
const workspaceStore = useWorkspaceStore()

const limit = ref(50)
const logs = ref<SvnLogEntry[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const selectedLog = ref<SvnLogEntry | null>(null)

const openWorkspace = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择 SVN 工作区目录'
  })

  if (selected) {
    const path = Array.isArray(selected) ? selected[0] : selected
    workspaceStore.setCurrentPath(path)
    loadLogs()
  }
}

const loadLogs = async () => {
  if (!workspaceStore.currentPath) return

  loading.value = true
  try {
    logs.value = await svnLog(workspaceStore.currentPath, limit.value)
  } catch (err) {
    console.error('加载日志失败:', err)
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
  return date.toLocaleString('zh-CN')
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

.no-workspace {
  padding: 40px 0;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.log-detail {
  padding: 10px 0;
}

.commit-message {
  background: #f5f7fa;
  padding: 15px;
  border-radius: 4px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
