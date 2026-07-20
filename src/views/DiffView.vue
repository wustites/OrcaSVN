<template>
  <div class="diff-view">
    <el-card class="diff-card">
      <template #header>
        <div class="card-header">
          <span class="view-title">
            <el-icon><Connection /></el-icon>
            {{ $t('diff.title') }}
          </span>
          <div class="header-actions">
            <template v-if="fileList.length > 0">
              <el-button-group class="nav-group">
                <el-button
                  size="small"
                  @click="goToPrevFile"
                  :disabled="currentFileIndex <= 0"
                >
                  <el-icon><ArrowLeft /></el-icon>
                  {{ $t('diff.previous') }}
                </el-button>
                <el-button
                  size="small"
                  @click="goToNextFile"
                  :disabled="currentFileIndex >= fileList.length - 1"
                >
                  {{ $t('diff.next') }}
                  <el-icon><ArrowRight /></el-icon>
                </el-button>
              </el-button-group>
              <el-tag type="info" effect="plain" size="small">
                {{ currentFileIndex + 1 }} / {{ fileList.length }}
              </el-tag>
              <el-button size="small" @click="backToCommit">
                <el-icon><Back /></el-icon>
                {{ $t('diff.backToCommit') }}
              </el-button>
            </template>
            <template v-else>
              <el-input
                v-model="currentPath"
                :placeholder="$t('diff.filePath')"
                clearable
                class="path-input"
              >
                <template #prepend>
                  <el-icon><Document /></el-icon>
                </template>
                <template #append>
                  <el-button @click="browseFile">{{ $t('common.browse') }}</el-button>
                </template>
              </el-input>
              <el-button @click="loadDiff" :loading="loading" type="primary">
                <el-icon><Connection /></el-icon>
                {{ $t('diff.compare') }}
              </el-button>
            </template>
          </div>
        </div>
      </template>

      <div class="diff-body">
        <div v-if="fileList.length > 0" class="file-sidebar">
          <div class="sidebar-header">
            <span class="sidebar-title">{{ $t('diff.fileList') }}</span>
            <span class="sidebar-count">{{ fileList.length }}</span>
          </div>
          <div class="sidebar-files">
            <div
              v-for="(file, idx) in fileList"
              :key="file"
              class="sidebar-file"
              :class="{ active: idx === currentFileIndex }"
              @click="switchToFile(idx)"
            >
              <span
                class="file-status-dot"
                :class="'dot-' + (fileStatusMap.get(file)?.status_code || 'modified')"
              ></span>
              <span class="file-name" :title="file">{{ fileDisplayNames[idx] || file }}</span>
            </div>
          </div>
        </div>

        <div class="diff-main">
          <div class="diff-options">
            <el-radio-group v-model="diffType" class="mode-switch" @change="handleDiffTypeChange">
              <el-radio-button label="working">{{ $t('diff.workingCopy') }}</el-radio-button>
              <el-radio-button label="revision">{{ $t('diff.betweenRevisions') }}</el-radio-button>
              <el-radio-button v-if="changeRev" label="change">
                {{ $t('diff.changeRevision', { revision: changeRev }) }}
              </el-radio-button>
            </el-radio-group>

            <div v-if="diffType === 'revision'" class="revision-inputs">
              <el-input
                v-model.number="oldRev"
                type="number"
                :min="0"
                :placeholder="$t('diff.oldRevision')"
                class="rev-input"
              />
              <span class="rev-separator">→</span>
              <el-input
                v-model.number="newRev"
                type="number"
                :min="0"
                :placeholder="$t('diff.newRevision')"
                class="rev-input"
              />
            </div>
          </div>

          <div v-if="!diffResult && !loading" class="empty-diff">
            <el-empty :description="$t('diff.selectFileAndCompare')">
              <template #image>
                <el-icon class="empty-icon"><Connection /></el-icon>
              </template>
            </el-empty>
          </div>

          <div v-else-if="loading" class="loading-diff">
            <el-skeleton :rows="20" animated />
          </div>

          <div v-else class="diff-content">
            <div class="diff-header">
              <div class="diff-tags">
                <el-tag class="path-tag" type="primary">
                  <el-icon><Document /></el-icon>
                  {{ $t('diff.file') }}：{{ diffResult?.path }}
                </el-tag>
                <el-tag v-if="diffResult?.old_revision && diffResult?.new_revision" type="info">
                  {{ $t('common.version') }}：{{ diffResult?.old_revision }} → {{ diffResult?.new_revision }}
                </el-tag>
              </div>
              <div class="diff-stats">
                <el-tag type="success" effect="plain" class="stat-tag">
                  <el-icon><Plus /></el-icon>
                  {{ diffStats.added }}
                </el-tag>
                <el-tag type="danger" effect="plain" class="stat-tag">
                  <el-icon><Minus /></el-icon>
                  {{ diffStats.removed }}
                </el-tag>
              </div>
            </div>

            <div class="diff-lines" role="table" :aria-label="$t('diff.title')">
              <div
                v-for="line in diffLines"
                :key="line.index"
                class="diff-row"
                :class="line.className"
                role="row"
              >
                <span class="diff-line-number" role="cell">{{ line.index }}</span>
                <span class="diff-marker" role="cell">{{ line.marker }}</span>
                <code class="diff-code" role="cell">{{ line.text }}</code>
              </div>
            </div>
          </div>
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter, onBeforeRouteLeave } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus/es/components/message/index'
import { svnDiff } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useI18n } from 'vue-i18n'
import { toWorkspaceRelativePath } from '@/utils/workspacePath'
import type { DiffResult, SvnStatus } from '@/types'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const workspaceStore = useWorkspaceStore()

type DiffLineType = 'added' | 'removed' | 'context' | 'meta'

interface DiffLineRow {
  index: number
  marker: string
  text: string
  className: string
  type: DiffLineType
}

const currentPath = ref('')
const diffType = ref<'working' | 'revision' | 'change'>('working')
const oldRev = ref<number>()
const newRev = ref<number>()
const changeRev = ref<number>()
const loading = ref(false)
const diffResult = ref<DiffResult | null>(null)
let requestGeneration = 0

const fileList = ref<string[]>([])
const fileDisplayNames = ref<string[]>([])
const currentFileIndex = ref(0)
const diffSource = ref<'commit' | 'log' | null>(null)

const fileStatusMap = computed(() => {
  const map = new Map<string, SvnStatus>()
  for (const s of workspaceStore.statusList) {
    map.set(s.path, s)
  }
  return map
})

const diffLines = computed<DiffLineRow[]>(() => {
  if (!diffResult.value?.diff) return []

  const lines = diffResult.value.diff.split('\n')
  return lines.map((line, index) => {
    if (line.startsWith('+') && !line.startsWith('+++')) {
      return {
        index: index + 1,
        marker: '+',
        text: line.slice(1),
        className: 'diff-added',
        type: 'added',
      }
    }

    if (line.startsWith('-') && !line.startsWith('---')) {
      return {
        index: index + 1,
        marker: '-',
        text: line.slice(1),
        className: 'diff-removed',
        type: 'removed',
      }
    }

    const isMeta = line.startsWith('@@') || line.startsWith('+++') || line.startsWith('---')
    return {
      index: index + 1,
      marker: isMeta ? '@' : '',
      text: line,
      className: isMeta ? 'diff-meta' : 'diff-context',
      type: isMeta ? 'meta' : 'context',
    }
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

const resolveCurrentFilePath = (): string | null => {
  const file = currentPath.value || route.query.path as string
  if (!file || !workspaceStore.currentPath) return null

  const relativePath = toWorkspaceRelativePath(file, workspaceStore.currentPath)
  if (relativePath === null) {
    ElMessage.warning(t('diff.fileOutsideWorkspace'))
    return null
  }
  if (!relativePath) return null

  currentPath.value = relativePath
  return relativePath
}

const browseFile = async () => {
  if (!workspaceStore.currentPath) return

  const selected = await open({
    directory: false,
    multiple: false,
    defaultPath: workspaceStore.currentPath,
    title: t('dialog.selectFile'),
  })

  if (!selected) return

  const selectedPath = Array.isArray(selected) ? selected[0] : selected
  const relativePath = toWorkspaceRelativePath(selectedPath, workspaceStore.currentPath)
  if (relativePath === null || !relativePath) {
    ElMessage.warning(t('diff.fileOutsideWorkspace'))
    return
  }
  currentPath.value = relativePath
}

const loadDiff = async () => {
  const file = resolveCurrentFilePath()
  if (!file || !workspaceStore.currentPath) return

  const generation = ++requestGeneration
  loading.value = true
  diffResult.value = null

  try {
    if (diffType.value === 'revision' && (oldRev.value === undefined || newRev.value === undefined)) {
      ElMessage.warning(t('diff.enterBothRevisions'))
      return
    }
    if (
      diffType.value === 'revision' &&
      (!Number.isInteger(oldRev.value) || !Number.isInteger(newRev.value) || oldRev.value! < 0 || newRev.value! < 0)
    ) {
      ElMessage.warning(t('diff.invalidRevision'))
      return
    }
    const old = diffType.value === 'revision' ? oldRev.value : changeRev.value
    const new_ = diffType.value === 'revision' ? newRev.value : undefined

    const result = await svnDiff(workspaceStore.currentPath, file, old, new_)
    if (generation === requestGeneration) diffResult.value = result
  } catch (err) {
    if (generation === requestGeneration) {
      diffResult.value = null
      ElMessage.error(`${t('common.error')}：${err}`)
    }
  } finally {
    if (generation === requestGeneration) loading.value = false
  }
}

const handleDiffTypeChange = (type: string | number | boolean) => {
  if (type !== 'change') changeRev.value = undefined
}

const switchToFile = (index: number) => {
  if (index < 0 || index >= fileList.value.length) return
  currentFileIndex.value = index
  currentPath.value = fileList.value[index]
  sessionStorage.setItem('orca_diff_files', JSON.stringify({
    source: diffSource.value,
    files: fileList.value,
    displayNames: fileDisplayNames.value,
    current: currentPath.value,
    index,
  }))
  const query: Record<string, string> = { path: fileList.value[index] }
  if (route.query.revision) query.revision = route.query.revision as string
  router.replace({ query })
}

const goToPrevFile = () => switchToFile(currentFileIndex.value - 1)
const goToNextFile = () => switchToFile(currentFileIndex.value + 1)

const backToCommit = () => {
  sessionStorage.removeItem('orca_diff_files')
  router.back()
}

onMounted(() => {
  const data = sessionStorage.getItem('orca_diff_files')
  if (data) {
    try {
      const parsed = JSON.parse(data)
      if (parsed.files?.length) {
        const files = parsed.files.filter((file: unknown): file is string => typeof file === 'string')
        const routePath = typeof route.query.path === 'string' ? route.query.path : ''
        const routeIndex = files.indexOf(routePath)
        if (routeIndex < 0) {
          sessionStorage.removeItem('orca_diff_files')
          return
        }

        fileList.value = files
        fileDisplayNames.value = Array.isArray(parsed.displayNames)
          ? parsed.displayNames.filter((name: unknown): name is string => typeof name === 'string')
          : []
        currentFileIndex.value = routeIndex
        diffSource.value = parsed.source === 'commit' || parsed.source === 'log' ? parsed.source : null
      }
    } catch {
      sessionStorage.removeItem('orca_diff_files')
    }
  }
})

onBeforeRouteLeave((to) => {
  sessionStorage.removeItem('orca_diff_files')
  if (diffSource.value === 'commit' && to.name !== 'commit') {
    sessionStorage.removeItem('orca_commit_form')
  }
  if (diffSource.value === 'log' && to.name !== 'log') {
    sessionStorage.removeItem('orca_log_selection')
  }
})

watch(
  () => [route.query.path, route.query.revision] as const,
  ([path, revision]) => {
    if (!path) return

    currentPath.value = path as string

    const idx = fileList.value.indexOf(path as string)
    if (idx >= 0) currentFileIndex.value = idx

    const parsedRevision = Number(revision)
    if (Number.isInteger(parsedRevision) && parsedRevision > 0) {
      diffType.value = 'change'
      changeRev.value = parsedRevision
    } else {
      diffType.value = 'working'
      changeRev.value = undefined
    }
    loadDiff()
  },
  { immediate: true }
)
</script>

<style scoped>
.diff-view {
  height: 100%;
}

.diff-card {
  height: 100%;
  border-radius: var(--app-radius-lg);
}

:deep(.diff-card > .el-card__body) {
  display: flex;
  flex-direction: column;
  height: calc(100% - 57px);
  min-height: 0;
}

.view-title {
  display: inline-flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  font-weight: 700;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
}

.path-input {
  width: min(420px, 42vw);
}

.diff-options {
  margin-bottom: var(--app-spacing-md);
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--app-spacing-md);
}

.mode-switch :deep(.el-radio-button__inner) {
  font-weight: 600;
}

.revision-inputs {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
}

.rev-input {
  width: 120px;
}

.rev-separator {
  color: var(--el-text-color-secondary);
  font-weight: 600;
}

.empty-diff,
.loading-diff {
  padding: var(--app-spacing-xl) 0;
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-icon {
  font-size: 64px;
  color: var(--el-text-color-placeholder);
}

.diff-content {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  margin-top: var(--app-spacing-md);
}

.diff-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: center;
  gap: var(--app-spacing);
  margin-bottom: var(--app-spacing);
}

.diff-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--app-spacing-sm);
}

.path-tag {
  max-width: min(720px, 100%);
}

.diff-stats {
  display: flex;
  gap: var(--app-spacing-sm);
}

.stat-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-weight: 600;
}

.diff-lines {
  flex: 1;
  min-height: 320px;
  background: #fbfbff;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
  overflow: auto;
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
  line-height: 1.5;
}

.diff-row {
  display: grid;
  grid-template-columns: 64px 34px minmax(max-content, 1fr);
  min-width: max-content;
  border-bottom: 1px solid rgba(226, 228, 238, 0.18);
  transition: background-color var(--app-transition-fast);
}

.diff-row:hover {
  filter: brightness(0.98);
}

.diff-line-number,
.diff-marker {
  user-select: none;
  color: #747789;
  background: rgba(241, 242, 251, 0.9);
  text-align: right;
}

.diff-line-number {
  padding: 3px 12px;
}

.diff-marker {
  padding: 3px 10px;
  text-align: center;
  font-weight: 800;
}

.diff-code {
  padding: 3px 12px;
  color: #20212a;
  white-space: pre;
}

.diff-added {
  background-color: #ecfdf3;
}

.diff-added .diff-marker,
.diff-added .diff-line-number {
  background: #dcfce7;
  color: #15803d;
}

.diff-removed {
  background-color: #fff1f2;
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
  font-weight: 800;
}

/* 暗色主题 */
.theme-dark .diff-lines {
  background: #1a1a2e;
  border-color: var(--md-sys-color-outline-variant);
}

.theme-dark .diff-row {
  border-bottom-color: rgba(143, 160, 174, 0.12);
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
  background-color: #052e16;
}

.theme-dark .diff-added .diff-marker,
.theme-dark .diff-added .diff-line-number {
  background: #052e16;
  color: #4ade80;
}

.theme-dark .diff-removed {
  background-color: #450a0a;
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

.diff-body {
  display: flex;
  flex: 1;
  min-height: 0;
  gap: 0;
}

.file-sidebar {
  width: 240px;
  flex-shrink: 0;
  border-right: 1px solid var(--md-sys-color-outline-variant);
  display: flex;
  flex-direction: column;
  background: var(--md-sys-color-surface-container);
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  font-size: 11px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
}

.sidebar-count {
  font-size: 10px;
  color: var(--el-text-color-placeholder);
}

.sidebar-files {
  flex: 1;
  overflow: auto;
}

.sidebar-file {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 12px;
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  transition: background var(--app-transition-fast);
}

.sidebar-file:hover {
  background: var(--md-sys-color-surface-container-high);
}

.sidebar-file.active {
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-on-primary-container);
}

.file-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-modified { background: #a16207; }
.dot-added { background: #15803d; }
.dot-deleted { background: #dc2626; }
.dot-unversioned { background: #6366f1; }
.dot-replaced { background: #a16207; }

.file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.diff-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.nav-group {
  margin-right: var(--app-spacing-sm);
}

.theme-dark .sidebar-file.active {
  background: var(--md-sys-color-primary-container);
}

@media (max-width: 860px) {
  .path-input {
    width: 100%;
  }

  .header-actions {
    width: 100%;
    flex-direction: column;
    align-items: stretch;
  }

  .diff-header {
    flex-direction: column;
    align-items: flex-start;
  }

  .file-sidebar {
    display: none;
  }
}
</style>
