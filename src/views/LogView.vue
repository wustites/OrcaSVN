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
            <span class="loaded-count">{{ $t('log.loadedCount', { count: logs.length }) }}</span>
            <el-button @click="reloadLogs(true)" :loading="loading" type="primary" size="small">
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

      <div v-else ref="logScroller" class="log-content" @scroll="handleScroll">
        <div class="log-filters">
          <el-select
            v-model="filters.author"
            :placeholder="$t('log.searchAuthor')"
            class="author-select"
            filterable
            clearable
            :disabled="authorOptions.length === 0"
            size="small"
          >
            <el-option
              v-for="option in authorOptions"
              :key="option.value"
              :label="option.label"
              :value="option.value"
            />
          </el-select>
          <el-input
            v-model="filters.keyword"
            :placeholder="$t('log.searchKeyword')"
            clearable
            size="small"
          />
          <el-date-picker
            v-model="dateRange"
            type="daterange"
            value-format="YYYY-MM-DD"
            format="YYYY/MM/DD"
            :start-placeholder="$t('log.dateFrom')"
            :end-placeholder="$t('log.dateTo')"
            range-separator="-"
            popper-class="log-date-range-dropdown"
            class="log-date-range"
            clearable
            unlink-panels
            :editable="false"
            size="small"
          />
          <el-button size="small" @click="resetFilters">
            <el-icon><RefreshLeft /></el-icon>
            {{ $t('common.reset') }}
          </el-button>
        </div>

        <el-table
          :data="filteredLogs"
          style="width: 100%"
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
          <el-table-column
            :label="$t('log.changedFiles')"
            width="150"
            align="center"
            class-name="changed-files-column"
            label-class-name="changed-files-header"
          >
            <template #default="{ row }">
              <el-tag size="small" effect="plain">
                {{ row.changed_paths?.length || 0 }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>
        <div class="load-more-state">
          <span v-if="loadingMore">{{ $t('log.loadingMore') }}</span>
          <span v-else-if="!hasMore && logs.length > 0">{{ $t('log.allLoaded') }}</span>
          <span v-else-if="hasMore">{{ $t('log.scrollForMore') }}</span>
        </div>
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

          <div class="changed-files-section">
            <h4 class="message-title">
              <el-icon><Document /></el-icon>
              {{ $t('log.changedFiles') }}
            </h4>
            <el-table
              :data="selectedLog.changed_paths || []"
              max-height="260"
              stripe
              class="changed-files-table"
              @row-dblclick="openChangedPathDiff"
            >
              <el-table-column :label="$t('commit.status')" width="96" align="center">
                <template #default="{ row }">
                  <el-tag size="small" :type="getActionTagType(row.action)">
                    {{ getActionLabel(row.action) }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column :label="$t('commit.file')" show-overflow-tooltip>
                <template #default="{ row }">
                  <span class="file-path">{{ row.path }}</span>
                </template>
              </el-table-column>
            </el-table>
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
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnCurrentUser, svnLog } from '@/api/svn'
import type { SvnLogEntry, SvnLogPath } from '@/types'
import { useI18n } from 'vue-i18n'
import { useWorkspace } from '@/composables/useWorkspace'
import { useSettings } from '@/composables/useSettings'
import { useRouter } from 'vue-router'

const { t, locale } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()
const { openWorkspace: openWorkspaceDialog } = useWorkspace()
const { settings } = useSettings()

const logs = ref<SvnLogEntry[]>([])
const loading = ref(false)
const loadingMore = ref(false)
const hasMore = ref(true)
const logScroller = ref<HTMLElement | null>(null)
let requestGeneration = 0
let authorRequestGeneration = 0
const dialogVisible = ref(false)
const selectedLog = ref<SvnLogEntry | null>(null)
const LOG_CACHE_TTL_MS = 5 * 60 * 1000
const MAX_CACHE_SIZE = 50
type LogCacheEntry = {
  entries: SvnLogEntry[]
  cachedAt: number
}
const logPageCache = new Map<string, LogCacheEntry>()
const logCacheVersion = ref(0)
const currentAuthor = ref('')

const getDefaultFilters = () => ({
  author: currentAuthor.value,
  keyword: '',
  dateFrom: '',
  dateTo: '',
})

const filters = reactive({
  ...getDefaultFilters(),
})
const pageSize = computed(() => Math.max(1, settings.logLimit || 50))

const dateRange = computed<string[]>({
  get: () => filters.dateFrom && filters.dateTo ? [filters.dateFrom, filters.dateTo] : [],
  set: (value: string[]) => {
    filters.dateFrom = value?.[0] || ''
    filters.dateTo = value?.[1] || ''
  },
})

const parseFilterDate = (value: string, endOfDay = false): Date | null => {
  const match = value.trim().match(/^(\d{4})[/-](\d{1,2})[/-](\d{1,2})$/)
  if (!match) return null

  const year = Number(match[1])
  const month = Number(match[2])
  const day = Number(match[3])
  const date = new Date(year, month - 1, day, endOfDay ? 23 : 0, endOfDay ? 59 : 0, endOfDay ? 59 : 0, endOfDay ? 999 : 0)
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day) return null
  return date
}

const getLogCacheKey = (path: string, limit: number, startRev?: number, endRev?: number, keyword?: string, dateFrom?: string, dateTo?: string) => {
  return `${path}|${limit}|${startRev ?? ''}|${endRev ?? ''}|${keyword ?? ''}|${dateFrom ?? ''}|${dateTo ?? ''}`
}

const getCachedLogPage = (path: string, limit: number, startRev?: number, endRev?: number, keyword?: string, dateFrom?: string, dateTo?: string) => {
  const key = getLogCacheKey(path, limit, startRev, endRev, keyword, dateFrom, dateTo)
  const cached = logPageCache.get(key)
  if (!cached) return null
  if (Date.now() - cached.cachedAt > LOG_CACHE_TTL_MS) {
    logPageCache.delete(key)
    return null
  }
  return cached.entries
}

const setCachedLogPage = (
  path: string,
  limit: number,
  entries: SvnLogEntry[],
  startRev?: number,
  endRev?: number,
  keyword?: string,
  dateFrom?: string,
  dateTo?: string
) => {
  // 缓存达到上限时淘汰最旧的条目
  if (logPageCache.size >= MAX_CACHE_SIZE) {
    let oldestKey: string | undefined
    let oldestTime = Infinity
    for (const [k, v] of logPageCache) {
      if (v.cachedAt < oldestTime) {
        oldestTime = v.cachedAt
        oldestKey = k
      }
    }
    if (oldestKey) logPageCache.delete(oldestKey)
  }

  const key = getLogCacheKey(path, limit, startRev, endRev, keyword, dateFrom, dateTo)
  logPageCache.set(key, {
    entries: entries.map(entry => ({
      ...entry,
      changed_paths: [...(entry.changed_paths || [])],
    })),
    cachedAt: Date.now(),
  })
  logCacheVersion.value += 1
}

const clearLogCacheForPath = (path: string) => {
  const prefix = `${path}|`
  let changed = false
  for (const key of logPageCache.keys()) {
    if (key.startsWith(prefix)) {
      logPageCache.delete(key)
      changed = true
    }
  }
  if (changed) logCacheVersion.value += 1
}

const getCachedAuthorsForPath = (path: string | null) => {
  if (!path) return []
  logCacheVersion.value

  const prefix = `${path}|`
  const authors = new Set<string>()
  for (const [key, cached] of logPageCache.entries()) {
    if (!key.startsWith(prefix)) continue
    cached.entries.forEach(entry => {
      if (entry.author) authors.add(entry.author)
    })
  }
  return [...authors]
}

const knownAuthors = computed(() => {
  const authors = new Set<string>()
  logs.value.forEach(entry => {
    if (entry.author) authors.add(entry.author)
  })
  getCachedAuthorsForPath(workspaceStore.currentPath).forEach(author => authors.add(author))
  return [...authors]
})

const authorOptions = computed(() => {
  const self = currentAuthor.value
  const otherAuthors = knownAuthors.value
    .filter(author => author && author !== self)
    .sort((a, b) => a.localeCompare(b, locale.value))

  return [
    ...(self ? [{ label: self, value: self }] : []),
    ...otherAuthors.map(author => ({ label: author, value: author })),
  ]
})

const loadCurrentAuthor = async (path: string) => {
  const generation = ++authorRequestGeneration
  try {
    const authUser = await svnCurrentUser(path)
    if (generation === authorRequestGeneration && path === workspaceStore.currentPath) {
      currentAuthor.value = authUser?.username || ''
    }
  } catch {
    if (generation === authorRequestGeneration && path === workspaceStore.currentPath) {
      currentAuthor.value = ''
    }
  }
}

const filteredLogs = computed(() => {
  const author = filters.author.trim().toLowerCase()
  const keyword = filters.keyword.trim().toLowerCase()
  const from = parseFilterDate(filters.dateFrom)
  const to = parseFilterDate(filters.dateTo, true)

  // 没有过滤条件时直接返回，避免遍历和字符串/Date计算
  if (!author && !keyword && !from && !to) return logs.value

  return logs.value.filter((entry) => {
    // 作者过滤（轻量，优先检查）
    if (author && entry.author.toLowerCase() !== author) return false

    // 关键字过滤（仅在需要时构造 searchable 字符串）
    if (keyword) {
      const paths = entry.changed_paths?.map((item) => item.path).join('\n') || ''
      const searchable = `${entry.message}\n${paths}\nr${entry.revision}`.toLowerCase()
      if (!searchable.includes(keyword)) return false
    }

    // 日期过滤（仅在需要时解析 Date）
    if (from || to) {
      const date = new Date(entry.date)
      if (from && date < from) return false
      if (to && date > to) return false
    }

    return true
  })
})

const openWorkspace = async () => {
  const success = await openWorkspaceDialog(t('dialog.selectSVNWorkspaceDirectory'))
  if (success) {
    reloadLogs()
  }
}

const fetchLogPage = async (generation: number, startRev?: number, refreshCache = false) => {
  if (!workspaceStore.currentPath || loading.value || loadingMore.value || !hasMore.value) return

  const requestedPath = workspaceStore.currentPath
  const limit = pageSize.value
  const endRev = startRev === undefined ? undefined : 1
  const initialLoad = startRev === undefined

  // 读取当前激活的过滤条件，传递给后端实现服务器端过滤
  const kw = filters.keyword.trim() || undefined
  const df = filters.dateFrom || undefined
  const dt = filters.dateTo || undefined

  if (initialLoad) loading.value = true
  else loadingMore.value = true
  try {
    const cachedBatch = refreshCache ? null : getCachedLogPage(requestedPath, limit, startRev, endRev, kw, df, dt)
    const batch = cachedBatch || await svnLog(requestedPath, limit, startRev, endRev, kw, df, dt)
    if (!cachedBatch) setCachedLogPage(requestedPath, limit, batch, startRev, endRev, kw, df, dt)
    if (generation !== requestGeneration || requestedPath !== workspaceStore.currentPath) return
    const knownRevisions = new Set(logs.value.map(entry => entry.revision))
    const newEntries = batch.filter(entry => !knownRevisions.has(entry.revision))
    logs.value = initialLoad ? batch : [...logs.value, ...newEntries]
    hasMore.value = batch.length >= limit && batch[batch.length - 1]?.revision !== 1
  } catch (err) {
    if (generation === requestGeneration) workspaceStore.setError(String(err))
  } finally {
    if (generation === requestGeneration) {
      loading.value = false
      loadingMore.value = false
    }
  }
}

const reloadLogs = async (refreshCache = false) => {
  if (!refreshCache && (loading.value || loadingMore.value)) return
  if (refreshCache && workspaceStore.currentPath) clearLogCacheForPath(workspaceStore.currentPath)

  const generation = ++requestGeneration
  logs.value = []
  hasMore.value = true
  loading.value = false
  loadingMore.value = false
  await fetchLogPage(generation, undefined, refreshCache)
  await nextTick()
  if (logScroller.value) logScroller.value.scrollTop = 0
}

const loadMore = async () => {
  const oldestRevision = logs.value[logs.value.length - 1]?.revision
  if (!oldestRevision || oldestRevision <= 1) {
    hasMore.value = false
    return
  }
  await fetchLogPage(requestGeneration, oldestRevision - 1)
}

const handleScroll = () => {
  const scroller = logScroller.value
  if (!scroller || !hasMore.value || loading.value || loadingMore.value) return
  if (scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 160) {
    loadMore()
  }
}

const handleRowClick = (row: SvnLogEntry) => {
  selectedLog.value = row
  dialogVisible.value = true
}

const openChangedPathDiff = (row: SvnLogPath) => {
  if (!selectedLog.value || !workspaceStore.svnInfo) return

  const repositoryRoot = workspaceStore.svnInfo.repository_root.replace(/\/+$/, '')
  const repositoryPath = row.path.startsWith('/') ? row.path : `/${row.path}`
  const revision = selectedLog.value.revision
  const pegRevision = row.action === 'D' ? Math.max(0, revision - 1) : revision
  dialogVisible.value = false
  router.push({
    name: 'diff',
    query: {
      path: `${repositoryRoot}${repositoryPath}@${pegRevision}`,
      revision: String(revision),
    },
  })
}

const resetFilters = () => {
  Object.assign(filters, getDefaultFilters())
}

const getActionLabel = (action: string) => {
  const key = `log.action${action || 'Unknown'}`
  const translated = t(key)
  return translated === key ? action || t('common.noData') : translated
}

const getActionTagType = (action: string) => {
  switch (action) {
    case 'A': return 'success'
    case 'D': return 'danger'
    case 'M': return 'warning'
    case 'R': return 'primary'
    default: return 'info'
  }
}

const formatDate = (dateStr: string): string => {
  if (!dateStr) return ''
  const date = new Date(dateStr)
  return date.toLocaleString(locale.value)
}

watch(
  () => workspaceStore.currentPath,
  (path, oldPath) => {
    if (path && path !== oldPath) {
      currentAuthor.value = ''
      loadCurrentAuthor(path)
      reloadLogs(true)
    }
    if (!path) {
      requestGeneration += 1
      authorRequestGeneration += 1
      logs.value = []
      currentAuthor.value = ''
      hasMore.value = true
      loading.value = false
      loadingMore.value = false
    }
  },
  { immediate: true }
)

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
  align-items: center;
  display: flex;
  gap: var(--app-spacing-sm);
}

.loaded-count {
  color: var(--el-text-color-secondary);
  font-size: 11px;
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

.log-filters {
  display: grid;
  grid-template-columns: minmax(120px, 1fr) minmax(180px, 2fr) 380px auto;
  gap: var(--app-spacing-sm);
  margin-bottom: var(--app-spacing);
}

.author-select {
  width: 100%;
}

.log-date-range {
  width: 100%;
  min-width: 0;
}

.log-date-range :deep(.el-range-input) {
  color: var(--el-text-color-primary);
}

.log-date-range :deep(.el-range-input::placeholder) {
  color: var(--el-text-color-placeholder);
}

.log-date-range :deep(.el-range-separator),
.log-date-range :deep(.el-range__icon),
.log-date-range :deep(.el-range__close-icon) {
  color: var(--el-text-color-secondary);
}

.log-table {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.log-table :deep(.changed-files-column .cell),
.log-table :deep(.changed-files-header .cell) {
  white-space: nowrap;
}

.load-more-state {
  padding: 12px;
  color: var(--el-text-color-secondary);
  font-size: 11px;
  text-align: center;
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

.changed-files-section {
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

.changed-files-table {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.changed-files-table :deep(.el-table__row) {
  cursor: pointer;
}

.file-path {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
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

  .log-filters {
    grid-template-columns: 1fr 1fr;
  }
}

@media (max-width: 640px) {
  .log-dialog {
    width: 90% !important;
  }

  .log-filters {
    grid-template-columns: 1fr;
  }
}
</style>
