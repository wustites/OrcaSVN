<template>
  <div class="update-view">
    <el-card class="update-card">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon><RefreshRight /></el-icon>
            {{ $t('update.title') }}
          </span>
          <div class="header-actions">
            <el-button size="small" @click="loadUpdateState(true)" :loading="loading">
              <el-icon><Refresh /></el-icon>
              {{ $t('update.check') }}
            </el-button>
            <el-button type="primary" size="small" @click="doUpdate" :loading="updating" :disabled="!workspaceStore.currentPath">
              <el-icon><RefreshRight /></el-icon>
              {{ $t('common.update') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-empty v-if="!workspaceStore.currentPath" :description="$t('log.openWorkspaceFirst')" class="empty-workspace" />

      <div v-else class="update-content">
        <div class="revision-strip">
          <div>
            <span>{{ $t('update.localRevision') }}</span>
            <strong>r{{ localRevision || '-' }}</strong>
          </div>
          <div>
            <span>{{ $t('update.remoteRevision') }}</span>
            <strong>r{{ remoteRevision || '-' }}</strong>
          </div>
          <div>
            <span>{{ $t('update.incomingCount') }}</span>
            <strong>{{ incomingLogs.length }}</strong>
          </div>
          <div>
            <span>{{ $t('update.localChangeCount') }}</span>
            <strong>{{ localChanges.length }}</strong>
          </div>
        </div>

        <section class="update-section">
          <div class="section-header">
            <h3>{{ $t('update.incomingTitle') }}</h3>
            <span>{{ $t('update.incomingHint') }}</span>
          </div>
          <el-table :data="incomingLogs" stripe class="update-table" max-height="280" row-key="revision">
            <el-table-column prop="revision" :label="$t('log.revision')" width="90" align="center">
              <template #default="{ row }">
                <el-tag type="primary" size="small" effect="plain">r{{ row.revision }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="author" :label="$t('log.author')" width="140" />
            <el-table-column :label="$t('log.date')" width="180">
              <template #default="{ row }">{{ formatDate(row.date) }}</template>
            </el-table-column>
            <el-table-column prop="message" :label="$t('log.commitMessage')" show-overflow-tooltip />
            <el-table-column :label="$t('log.changedFiles')" width="110" align="center">
              <template #default="{ row }">{{ row.changed_paths?.length || 0 }}</template>
            </el-table-column>
          </el-table>
          <div v-if="!loading && incomingLogs.length === 0" class="empty-line">
            {{ $t('update.noIncoming') }}
          </div>
        </section>

        <div class="list-divider"></div>

        <section class="update-section">
          <div class="section-header">
            <h3>{{ $t('update.localTitle') }}</h3>
            <span>{{ $t('update.localHint') }}</span>
          </div>
          <el-table :data="localChanges" stripe class="update-table" max-height="280" row-key="path">
            <el-table-column prop="status_code" :label="$t('commit.status')" width="120" align="center">
              <template #default="{ row }">
                <span class="status-badge" :class="getStatusClass(row.status_code)">
                  {{ $t(getStatusLabelKey(row.status_code)) }}
                </span>
              </template>
            </el-table-column>
            <el-table-column prop="path" :label="$t('commit.file')" show-overflow-tooltip>
              <template #default="{ row }">
                <button class="file-link" type="button" @click="viewDiff(row.path)">
                  {{ row.path }}
                </button>
              </template>
            </el-table-column>
            <el-table-column :label="$t('common.action')" width="90" align="center">
              <template #default="{ row }">
                <el-button text size="small" @click="viewDiff(row.path)">
                  {{ $t('common.diff') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
          <div v-if="!loading && localChanges.length === 0" class="empty-line">
            {{ $t('workspace.noChanges') }}
          </div>
        </section>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import { svnLog, svnRemoteInfo, svnUpdate } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useWorkspace } from '@/composables/useWorkspace'
import { getStatusClass, getStatusLabelKey } from '@/composables/useSvnStatus'
import { useSettings } from '@/composables/useSettings'
import type { SvnLogEntry } from '@/types'

type UpdateStateCache = {
  remoteRevision: number
  incomingLogs: SvnLogEntry[]
}

const updateStateCache = new Map<string, UpdateStateCache>()

const router = useRouter()
const { t, locale } = useI18n()
const workspaceStore = useWorkspaceStore()
const { refreshStatus } = useWorkspace()
const { settings } = useSettings()

const loading = ref(false)
const updating = ref(false)
const incomingLogs = ref<SvnLogEntry[]>([])
const remoteRevision = ref(0)
let updateStateTimer: number | undefined
const updateStateIntervalMs = 30_000

const localRevision = computed(() => workspaceStore.svnInfo?.revision || 0)
const localChanges = computed(() => workspaceStore.statusList.filter(status => status.status_code !== 'normal' && status.status_code !== ''))

const formatDate = (date: string) => date ? new Date(date).toLocaleString(locale.value) : ''

const applyCachedState = (path: string) => {
  const cached = updateStateCache.get(path)
  if (!cached) return false

  remoteRevision.value = cached.remoteRevision
  incomingLogs.value = cached.incomingLogs.map(entry => ({
    ...entry,
    changed_paths: [...(entry.changed_paths || [])],
  }))
  return true
}

const loadUpdateState = async (force = false) => {
  if (!workspaceStore.currentPath || loading.value || (!force && updating.value)) return
  const requestedPath = workspaceStore.currentPath
  loading.value = true
  try {
    await refreshStatus()
    const info = await svnRemoteInfo(requestedPath)
    remoteRevision.value = info.revision
    const local = workspaceStore.svnInfo?.revision || 0
    if (info.revision > local) {
      incomingLogs.value = await svnLog(requestedPath, settings.logLimit || 50, info.revision, local + 1)
    } else {
      incomingLogs.value = []
    }
    updateStateCache.set(requestedPath, {
      remoteRevision: remoteRevision.value,
      incomingLogs: incomingLogs.value.map(entry => ({
        ...entry,
        changed_paths: [...(entry.changed_paths || [])],
      })),
    })
  } catch (err) {
    workspaceStore.setError(String(err))
    ElMessage.error(`${t('common.error')}：${err}`)
  } finally {
    loading.value = false
  }
}

const doUpdate = async () => {
  if (!workspaceStore.currentPath) return
  updating.value = true
  try {
    await svnUpdate(workspaceStore.currentPath)
    await loadUpdateState(true)
    ElMessage.success(`${t('common.update')} ${t('common.success')}`)
  } catch (err) {
    workspaceStore.setError(String(err))
    ElMessage.error(`${t('common.error')}：${err}`)
  } finally {
    updating.value = false
  }
}

const viewDiff = (path: string) => {
  router.push({ name: 'diff', query: { path } })
}

onMounted(() => {
  if (workspaceStore.currentPath && !applyCachedState(workspaceStore.currentPath)) {
    loadUpdateState()
  }
  updateStateTimer = window.setInterval(loadUpdateState, updateStateIntervalMs)
})

onUnmounted(() => {
  if (updateStateTimer !== undefined) {
    window.clearInterval(updateStateTimer)
  }
})

watch(
  () => workspaceStore.currentPath,
  (path) => {
    incomingLogs.value = []
    remoteRevision.value = 0
    if (path && !applyCachedState(path)) {
      loadUpdateState()
    }
  }
)
</script>

<style scoped>
.update-view {
  height: 100%;
}

.update-card {
  min-height: 100%;
  border-radius: var(--app-radius-lg);
}

.card-header,
.header-actions {
  display: flex;
  align-items: center;
}

.card-header {
  justify-content: space-between;
}

.card-title {
  display: inline-flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  font-weight: 700;
}

.header-actions {
  gap: var(--app-spacing-sm);
}

.empty-workspace {
  height: 420px;
}

.update-content {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.revision-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(120px, 1fr));
  gap: 8px;
}

.revision-strip div {
  padding: 10px 12px;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: 6px;
  background: var(--el-fill-color-light);
}

.revision-strip span,
.section-header span {
  display: block;
  color: var(--el-text-color-secondary);
  font-size: 11px;
}

.revision-strip strong {
  display: block;
  margin-top: 4px;
  color: var(--el-text-color-primary);
  font-size: 18px;
}

.update-section {
  min-height: 0;
}

.section-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 8px;
}

.section-header h3 {
  margin: 0;
  color: var(--el-text-color-primary);
  font-size: 15px;
}

.update-table {
  border-radius: 6px;
  overflow: hidden;
}

.list-divider {
  height: 1px;
  background: var(--md-sys-color-outline-variant);
}

.empty-line {
  padding: 12px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  text-align: center;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 72px;
  height: 24px;
  padding: 0 8px;
  border-radius: var(--app-radius-full);
  font-size: 11px;
  font-weight: 700;
}

.file-link {
  max-width: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--md-sys-color-primary);
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 12px;
  text-align: left;
  cursor: pointer;
}

.file-link:hover {
  text-decoration: underline;
}

@media (max-width: 860px) {
  .revision-strip {
    grid-template-columns: repeat(2, minmax(120px, 1fr));
  }
}
</style>
