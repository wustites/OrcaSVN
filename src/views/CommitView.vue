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

        <el-form class="commit-form" label-position="top">
          <el-form-item :label="$t('commit.selectFiles')" class="file-selection">
            <el-table 
              ref="commitTable"
              :data="changedFiles" 
              style="width: 100%" 
              @selection-change="handleSelectionChange"
              stripe
              highlight-current-row
              max-height="300"
              class="file-table"
              row-key="path"
            >
              <el-table-column type="selection" width="50" align="center" :reserve-selection="true" />
              <el-table-column prop="status_code" :label="$t('commit.status')" width="120" align="center">
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
            </el-table>
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
          </el-form-item>

          <el-form-item class="form-actions">
            <el-button
              type="primary"
              @click="doCommit"
              :loading="loading"
              :disabled="!commitMessage || changedFiles.length === 0"
            >
              <el-icon><Upload /></el-icon>
              {{ $t('common.commit') }}
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
import { ref, computed, nextTick, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnCommit } from '@/api/svn'
import { useI18n } from 'vue-i18n'
import { getStatusClass, getStatusLabelKey } from '@/composables/useSvnStatus'
import { useWorkspace } from '@/composables/useWorkspace'
import type { SvnStatus } from '@/types'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog } = useWorkspace()

const commitTable = ref()
const selectedFiles = ref<string[]>([])
const commitMessage = ref('')
const loading = ref(false)
const output = ref('')

const committableStatuses = new Set(['added', 'modified', 'deleted', 'replaced'])

const changedFiles = computed(() => {
  return workspaceStore.statusList.filter(
    s => committableStatuses.has(s.status_code) || s.prop_status === 'modified'
  )
})

const handleSelectionChange = (rows: SvnStatus[]) => {
  selectedFiles.value = rows.map(f => f.path)
}

const routeSelectedFiles = computed(() => {
  const files = route.query.files
  if (Array.isArray(files)) {
    return files.filter((file): file is string => typeof file === 'string')
  }
  return typeof files === 'string' ? [files] : []
})

const routeCommittableFiles = computed(() => {
  const selected = new Set(routeSelectedFiles.value)
  return changedFiles.value
    .filter((file) => selected.has(file.path))
    .map((file) => file.path)
})

const applyRouteSelection = async () => {
  const files = routeSelectedFiles.value
  if (files.length === 0) return

  selectedFiles.value = routeCommittableFiles.value
  await nextTick()
  if (!commitTable.value) return

  commitTable.value?.clearSelection()

  changedFiles.value.forEach((file) => {
    if (files.includes(file.path)) {
      commitTable.value?.toggleRowSelection(file, true)
    }
  })
}

onMounted(applyRouteSelection)
watch(changedFiles, applyRouteSelection, { immediate: true, flush: 'post' })
watch(() => route.query.files, applyRouteSelection, { flush: 'post' })

const openWorkspace = async () => {
  const success = await openWorkspaceDialog(t('dialog.selectSVNWorkspaceDirectory'))
  if (success) {
    router.push({ name: 'workspace' })
  }
}

const doCommit = async () => {
  if (!workspaceStore.currentPath || !commitMessage.value) {
    return
  }

  loading.value = true
  output.value = ''

  try {
    const selected = selectedFiles.value.length > 0 ? selectedFiles.value : routeCommittableFiles.value
    const files = selected.length > 0 ? selected : undefined
    const result = await svnCommit(workspaceStore.currentPath, commitMessage.value, files)
    output.value = result.output

    setTimeout(() => {
      router.push({ name: 'workspace' })
    }, 1500)
  } catch (err) {
    output.value = `${t('common.error')}：${err}`
  } finally {
    loading.value = false
  }
}

const resetForm = () => {
  selectedFiles.value = []
  commitMessage.value = ''
  output.value = ''
  commitTable.value?.clearSelection()
}
</script>

<style scoped>
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
