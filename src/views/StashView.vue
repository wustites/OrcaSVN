<template>
  <div class="stash-view">
    <el-card class="stash-card animate-fade-in">
      <template #header>
        <div class="card-header">
          <div class="card-title-wrap">
            <span class="card-title">
              <el-icon><Archive /></el-icon>
              {{ $t('stash.title') }}
            </span>
            <span class="card-subtitle">{{ $t('stash.subtitle') }}</span>
          </div>
          <div class="header-actions">
            <el-button size="small" @click="refreshWorkspace" :loading="refreshing">
              <el-icon><Refresh /></el-icon>
              {{ $t('common.refresh') }}
            </el-button>
            <el-button type="primary" size="small" @click="openCreateDialog" :disabled="!hasStashableChanges">
              <el-icon><Plus /></el-icon>
              {{ $t('stash.create') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-empty v-if="!workspaceStore.currentPath" :description="$t('log.openWorkspaceFirst')" class="empty-state" />

      <div v-else class="stash-content">
        <el-alert
          v-if="hasStashableChanges"
          :title="$t('stash.workingCopyHint', { count: stashableFiles.length + unversionedFiles.length })"
          type="info"
          :closable="false"
          show-icon
        />
        <el-alert
          v-else-if="hasChanges"
          :title="$t('stash.unsupportedChanges')"
          type="warning"
          :closable="false"
          show-icon
        />

        <el-empty v-if="stashes.length === 0" :description="$t('stash.empty')" class="empty-state">
          <el-button type="primary" :disabled="!hasStashableChanges" @click="openCreateDialog">
            {{ $t('stash.create') }}
          </el-button>
        </el-empty>

        <div v-else class="stash-list">
          <article v-for="(entry, index) in stashes" :key="entry.id" class="stash-entry">
            <div class="stash-index">{{ index + 1 }}</div>
            <div class="stash-entry-main">
              <div class="stash-entry-heading">
                <strong>{{ entry.name }}</strong>
                <el-tag size="small" effect="plain">{{ formatDate(entry.createdAt) }}</el-tag>
              </div>
              <div class="stash-entry-meta">
                <span><el-icon><Document /></el-icon>{{ $t('stash.files', { count: entry.files.length }) }}</span>
                <span><el-icon><List /></el-icon>{{ $t('stash.hunks', { count: entry.hunkCount }) }}</span>
                <span v-if="entry.unversionedFiles?.length"><el-icon><Plus /></el-icon>{{ $t('stash.unversionedFiles', { count: unversionedRootCount(entry) }) }}</span>
                <span class="stash-path" :title="entry.workspacePath">{{ entry.workspacePath }}</span>
              </div>
            </div>
            <div class="stash-entry-actions">
              <el-button size="small" :loading="applyingEntryId === entry.id && applyingMode === 'apply'" :disabled="applyingEntryId !== null" @click="applyStash(entry)">{{ $t('stash.apply') }}</el-button>
              <el-button size="small" type="primary" :loading="applyingEntryId === entry.id && applyingMode === 'pop'" :disabled="applyingEntryId !== null" @click="popStash(entry)">{{ $t('stash.pop') }}</el-button>
              <el-dropdown trigger="click" @command="handleEntryDropdown(entry)">
                <el-button size="small" text>
                  <el-icon><MoreFilled /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="copy" :disabled="!entry.patch"><el-icon><CopyDocument /></el-icon>{{ $t('stash.copy') }}</el-dropdown-item>
                    <el-dropdown-item command="export" :disabled="!entry.patch"><el-icon><Download /></el-icon>{{ $t('stash.export') }}</el-dropdown-item>
                    <el-dropdown-item divided command="delete"><el-icon><Delete /></el-icon>{{ $t('common.delete') }}</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </article>
        </div>
      </div>
    </el-card>

    <el-dialog v-model="createDialogVisible" :title="$t('stash.createTitle')" width="min(1080px, 94vw)" top="4vh" destroy-on-close>
      <div class="create-dialog">
        <div class="create-toolbar">
          <div>
            <strong>{{ $t('stash.selectHunks') }}</strong>
            <span>{{ $t('stash.selectHunksHint') }}</span>
          </div>
          <div class="select-actions">
            <span class="selection-count">{{ $t('stash.selectedItems', { count: selectedItemCount }) }}</span>
            <el-button text size="small" @click="selectAllHunks">{{ $t('stash.selectAll') }}</el-button>
            <el-button text size="small" @click="clearSelectedHunks">{{ $t('stash.clearSelection') }}</el-button>
          </div>
        </div>

        <el-skeleton v-if="loadingDiffs" :rows="8" animated />
        <el-empty v-else-if="diffFiles.length === 0 && unversionedFiles.length === 0" :description="$t('stash.noDiffs')" />
        <div v-else class="diff-file-list">
          <section v-if="unversionedFiles.length" class="unversioned-section">
            <div class="section-heading">
              <div>
                <strong>{{ $t('stash.unversionedTitle') }}</strong>
                <span>{{ $t('stash.unversionedHint') }}</span>
              </div>
              <el-tag type="warning" effect="plain">{{ $t('stash.files', { count: unversionedFiles.length }) }}</el-tag>
            </div>
            <div v-for="file in unversionedFiles" :key="file.path" class="unversioned-file" :class="{ selected: selectedUnversioned.includes(file.path) }" @click="onUnversionedToggle(file.path, !selectedUnversioned.includes(file.path))">
              <div class="unversioned-file-header">
                <el-checkbox :model-value="selectedUnversioned.includes(file.path)" @click.stop @change="onUnversionedToggle(file.path, $event)" />
                <el-icon><Document /></el-icon>
                <span class="unversioned-file-path">{{ file.path }}</span>
                <el-tag size="small" type="warning" effect="light">{{ $t('status.unversioned') }}</el-tag>
              </div>
              <div class="unversioned-preview" @click.stop>
                <template v-for="entry in previewEntries(file.path)" :key="entry.path">
                  <div v-if="entry.kind === 'directory'" class="unversioned-preview-note">{{ entry.path }}/ · {{ $t('stash.directory') }}</div>
                  <template v-else>
                    <div v-if="previewEntries(file.path).length > 1" class="unversioned-preview-path">{{ entry.path }}</div>
                    <pre v-if="decodePreview(entry) !== null" class="hunk-lines unversioned-lines"><span v-for="(line, lineIndex) in decodePreview(entry)!.split('\n')" :key="lineIndex" class="line-added">+ {{ line }}</span></pre>
                    <div v-else class="unversioned-preview-note">{{ $t('stash.binaryFile') }} · {{ formatContentSize(entry.contentBase64) }}</div>
                  </template>
                </template>
              </div>
            </div>
          </section>
          <section v-for="file in diffFiles" :key="file.path" class="diff-file">
            <div class="diff-file-header" :class="{ selected: file.hunks.some(hunk => selectedHunks.includes(hunk.id)) }" @click="toggleFile(file, !file.hunks.every(hunk => selectedHunks.includes(hunk.id)))">
              <el-checkbox
                :model-value="file.hunks.every(hunk => selectedHunks.includes(hunk.id))"
                :indeterminate="file.hunks.some(hunk => selectedHunks.includes(hunk.id)) && !file.hunks.every(hunk => selectedHunks.includes(hunk.id))"
                @click.stop
                @change="onFileToggle(file, $event)"
              >
                <span class="diff-file-path">{{ file.path }}</span>
              </el-checkbox>
              <span class="hunk-count">{{ $t('stash.hunks', { count: file.hunks.length }) }}</span>
            </div>
            <div v-for="hunk in file.hunks" :key="hunk.id" class="hunk-card" :class="{ selected: selectedHunks.includes(hunk.id) }" @click="toggleHunk(hunk.id, !selectedHunks.includes(hunk.id))">
              <div class="hunk-heading">
                <el-checkbox :model-value="selectedHunks.includes(hunk.id)" @click.stop @change="onHunkToggle(hunk.id, $event)">
                  <code>{{ hunk.header }}</code>
                </el-checkbox>
                <span class="hunk-stats">
                  <span class="added">+{{ hunk.added }}</span>
                  <span class="removed">-{{ hunk.removed }}</span>
                </span>
              </div>
              <pre class="hunk-lines"><span v-for="(line, lineIndex) in hunk.lines" :key="lineIndex" :class="lineClass(line)">{{ line }}</span></pre>
            </div>
          </section>
        </div>

        <div class="stash-name-field">
          <label for="stash-name">{{ $t('stash.name') }}</label>
          <el-input id="stash-name" v-model="stashName" :placeholder="$t('stash.namePlaceholder')" size="large" clearable maxlength="120" show-word-limit />
        </div>
      </div>
      <template #footer>
        <el-button @click="createDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="creating" :disabled="selectedItemCount === 0" @click="createStash">
          {{ $t('stash.saveAndHide') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { deleteUnversioned, readUnversionedFiles, restoreUnversionedFiles, svnApplyPatch, svnDiff } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useStashStore } from '@/stores/stash'
import { useWorkspace } from '@/composables/useWorkspace'
import { buildPatch, createStashId, parseUnifiedDiff } from '@/utils/stash'
import type { StashEntry, StashFile, StashUnversionedEntry } from '@/types'
import { Archive, CopyDocument, Delete, Document, Download, List, MoreFilled, Plus, Refresh } from '@/components/icons/materialIcons'

const { t, locale } = useI18n()
const workspaceStore = useWorkspaceStore()
const stashStore = useStashStore()
const { refreshStatus } = useWorkspace()

const createDialogVisible = ref(false)
const loadingDiffs = ref(false)
const creating = ref(false)
const refreshing = ref(false)
const diffFiles = ref<StashFile[]>([])
const selectedHunks = ref<string[]>([])
const selectedUnversioned = ref<string[]>([])
const unversionedPreview = ref<StashUnversionedEntry[]>([])
const stashName = ref('')
const applyingEntryId = ref<string | null>(null)
const applyingMode = ref<'apply' | 'pop' | null>(null)

const changedFiles = computed(() => workspaceStore.statusList.filter(status => status.status_code !== 'normal' && status.status_code !== ''))
const hasChanges = computed(() => changedFiles.value.length > 0)
const stashableStatuses = new Set(['modified', 'added', 'deleted', 'replaced'])
const stashableFiles = computed(() => changedFiles.value.filter(file => stashableStatuses.has(file.status_code)))
const isDescendant = (path: string, parent: string) => {
  const target = path.replace(/\\/g, '/')
  const root = parent.replace(/\\/g, '/').replace(/\/$/, '')
  return target !== root && target.startsWith(`${root}/`)
}
const unversionedFiles = computed(() => changedFiles.value
  .filter(file => file.status_code === 'unversioned')
  .filter((file, _index, files) => !files.some(parent => isDescendant(file.path, parent.path))))
const hasStashableChanges = computed(() => stashableFiles.value.length > 0 || unversionedFiles.value.length > 0)
const selectedItemCount = computed(() => selectedHunks.value.length + selectedUnversioned.value.length)
const stashes = computed(() => stashStore.currentEntries(workspaceStore.currentPath))

const formatDate = (date: string) => new Date(date).toLocaleString(locale.value)

const lineClass = (line: string) => ({
  'line-added': line.startsWith('+') && !line.startsWith('+++'),
  'line-removed': line.startsWith('-') && !line.startsWith('---'),
  'line-meta': line.startsWith('\\'),
})

const openCreateDialog = async () => {
  if (!workspaceStore.currentPath || loadingDiffs.value) return
  const workspacePath = workspaceStore.currentPath
  createDialogVisible.value = true
  stashName.value = `WIP on ${new Date().toLocaleString(locale.value)}`
  selectedHunks.value = []
  selectedUnversioned.value = []
  unversionedPreview.value = []
  loadingDiffs.value = true
  try {
    const [parsed, preview] = await Promise.all([Promise.all(stashableFiles.value.map(async file => {
      const result = await svnDiff(workspacePath, file.path)
      return parseUnifiedDiff(file.path, result.diff)
    })), unversionedFiles.value.length
      ? readUnversionedFiles(workspacePath, unversionedFiles.value.map(file => file.path))
      : Promise.resolve([])])
    if (workspaceStore.currentPath !== workspacePath || !createDialogVisible.value) return
    diffFiles.value = parsed.filter((file): file is StashFile => file !== null)
    unversionedPreview.value = preview
    selectAllHunks()
    selectedUnversioned.value = unversionedFiles.value.map(file => file.path)
  } catch (err) {
    diffFiles.value = []
    ElMessage.error(`${t('common.error')}：${err}`)
  } finally {
    loadingDiffs.value = false
  }
}

const toggleHunk = (id: string, checked: boolean) => {
  selectedHunks.value = checked
    ? [...new Set([...selectedHunks.value, id])]
    : selectedHunks.value.filter(item => item !== id)
}

const onHunkToggle = (id: string, value: unknown) => toggleHunk(id, Boolean(value))

const toggleFile = (file: StashFile, checked: boolean) => {
  const ids = new Set(selectedHunks.value)
  file.hunks.forEach(hunk => checked ? ids.add(hunk.id) : ids.delete(hunk.id))
  selectedHunks.value = [...ids]
}

const onFileToggle = (file: StashFile, value: unknown) => toggleFile(file, Boolean(value))

const selectAllHunks = () => {
  selectedHunks.value = diffFiles.value.flatMap(file => file.hunks.map(hunk => hunk.id))
  selectedUnversioned.value = unversionedFiles.value.map(file => file.path)
}

const clearSelectedHunks = () => {
  selectedHunks.value = []
  selectedUnversioned.value = []
}

const onUnversionedToggle = (path: string, value: unknown) => {
  selectedUnversioned.value = Boolean(value)
    ? [...new Set([...selectedUnversioned.value, path])]
    : selectedUnversioned.value.filter(item => item !== path)
}

const createStash = async () => {
  if (!workspaceStore.currentPath || selectedItemCount.value === 0) return
  const workspacePath = workspaceStore.currentPath
  const patch = buildPatch(diffFiles.value, new Set(selectedHunks.value))
  if (!patch && selectedUnversioned.value.length === 0) {
    ElMessage.warning(t('stash.noSelection'))
    return
  }

  let capturedUnversioned
  try {
    capturedUnversioned = selectedUnversioned.value.length
      ? await readUnversionedFiles(workspacePath, selectedUnversioned.value)
      : []
  } catch (err) {
    ElMessage.error(`${t('stash.createFailed')}：${err}`)
    return
  }

  const entry: StashEntry = {
    id: createStashId(),
    name: stashName.value.trim() || t('stash.defaultName'),
    workspacePath,
    createdAt: new Date().toISOString(),
    patch,
    files: [...new Set([
      ...diffFiles.value.filter(file => file.hunks.some(hunk => selectedHunks.value.includes(hunk.id))).map(file => file.path),
      ...selectedUnversioned.value,
    ])],
    hunkCount: selectedHunks.value.length,
    unversionedFiles: capturedUnversioned,
  }

  creating.value = true
  try {
    // Save the only durable copy before removing changes from the working copy.
    stashStore.addEntry(entry)
  } catch (err) {
    ElMessage.error(`${t('stash.createFailed')}：${err}`)
    creating.value = false
    return
  }

  try {
    if (patch) await svnApplyPatch(workspacePath, patch, true)
    if (selectedUnversioned.value.length) await deleteUnversioned(workspacePath, selectedUnversioned.value)
    createDialogVisible.value = false
    await refreshStatus()
    ElMessage.success(t('stash.created'))
  } catch (err) {
    // The patch is already stored, so retain it and keep the source changes.
    createDialogVisible.value = false
    await refreshStatus()
    ElMessage.warning(`${t('stash.savedButNotHidden')}：${err}`)
  } finally {
    creating.value = false
  }
}

const applyEntry = async (entry: StashEntry, removeAfterApply: boolean) => {
  if (!workspaceStore.currentPath || workspaceStore.currentPath !== entry.workspacePath || applyingEntryId.value) return
  const workspacePath = workspaceStore.currentPath
  applyingEntryId.value = entry.id
  applyingMode.value = removeAfterApply ? 'pop' : 'apply'
  try {
    if (entry.unversionedFiles?.length) await restoreUnversionedFiles(workspacePath, entry.unversionedFiles, true)
    if (entry.patch) await svnApplyPatch(workspacePath, entry.patch)
    if (entry.unversionedFiles?.length) await restoreUnversionedFiles(workspacePath, entry.unversionedFiles)
    if (removeAfterApply) stashStore.removeEntry(entry.id)
    await refreshStatus()
    ElMessage.success(removeAfterApply ? t('stash.popped') : t('stash.applied'))
  } catch (err) {
    ElMessage.error(`${t('stash.applyFailed')}：${err}`)
  } finally {
    applyingEntryId.value = null
    applyingMode.value = null
  }
}

const applyStash = (entry: StashEntry) => applyEntry(entry, false)
const popStash = (entry: StashEntry) => applyEntry(entry, true)

const unversionedRootCount = (entry: StashEntry) => entry.files.filter(path =>
  entry.unversionedFiles?.some(file => file.path === path)
).length

const copyEntry = async (entry: StashEntry) => {
  try {
    await navigator.clipboard.writeText(entry.patch)
    ElMessage.success(t('stash.copied'))
  } catch (err) {
    ElMessage.error(`${t('stash.copyFailed')}：${err}`)
  }
}

const exportEntry = async (entry: StashEntry) => {
  const path = await save({
    defaultPath: `${entry.name.replace(/[^\w\-. ]+/g, '_')}.patch`,
    filters: [{ name: 'Patch', extensions: ['patch', 'diff'] }],
  })
  if (!path) return
  try {
    await writeTextFile(path, entry.patch)
    ElMessage.success(t('stash.exported'))
  } catch (err) {
    ElMessage.error(`${t('stash.exportFailed')}：${err}`)
  }
}

const deleteEntry = async (entry: StashEntry) => {
  try {
    await ElMessageBox.confirm(t('stash.deleteConfirm', { name: entry.name }), t('common.confirm'), {
      type: 'warning',
      confirmButtonText: t('common.delete'),
      cancelButtonText: t('common.cancel'),
    })
    stashStore.removeEntry(entry.id)
  } catch (err) {
    if (err === 'cancel' || err === 'close') return
    ElMessage.error(`${t('stash.deleteFailed')}：${err}`)
  }
}

const handleEntryCommand = (command: string | number | boolean, entry: StashEntry) => {
  if (command === 'copy') void copyEntry(entry)
  if (command === 'export') void exportEntry(entry)
  if (command === 'delete') void deleteEntry(entry)
}

const handleEntryDropdown = (entry: StashEntry) => (command: string | number | boolean) => handleEntryCommand(command, entry)

const refreshWorkspace = async () => {
  refreshing.value = true
  try {
    await refreshStatus()
  } finally {
    refreshing.value = false
  }
}

onMounted(() => {
  if (workspaceStore.currentPath && !workspaceStore.statusList.length) void refreshWorkspace()
})

const previewEntries = (root: string) => unversionedPreview.value.filter(entry =>
  entry.path === root || isDescendant(entry.path, root)
)

const decodePreview = (entry: StashUnversionedEntry) => {
  if (!entry.contentBase64) return entry.kind === 'file' ? '' : null
  try {
    const bytes = Uint8Array.from(atob(entry.contentBase64), char => char.charCodeAt(0))
    if (bytes.includes(0)) return null
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    return null
  }
}

const formatContentSize = (content?: string) => `${Math.floor((content?.length ?? 0) * 3 / 4)} B`

watch(() => workspaceStore.currentPath, () => {
  createDialogVisible.value = false
  diffFiles.value = []
  selectedHunks.value = []
  selectedUnversioned.value = []
  unversionedPreview.value = []
})
</script>

<style scoped>
.stash-view,
.stash-card {
  height: 100%;
}

.stash-card {
  border-radius: var(--app-radius-lg);
}

:deep(.stash-card > .el-card__body) {
  height: calc(100% - 57px);
  min-height: 0;
  overflow: auto;
}

.card-header,
.header-actions,
.card-title,
.card-title-wrap,
.stash-entry-heading,
.stash-entry-meta,
.stash-entry-actions,
.create-toolbar,
.select-actions,
.diff-file-header,
.hunk-heading,
.hunk-stats {
  display: flex;
  align-items: center;
}

.card-header,
.create-toolbar,
.diff-file-header {
  justify-content: space-between;
}

.card-title-wrap {
  gap: var(--app-spacing-md);
}

.card-title {
  gap: var(--app-spacing-sm);
  font-weight: 700;
}

.card-subtitle,
.create-toolbar span,
.stash-entry-meta,
.hunk-count {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.header-actions,
.stash-entry-actions,
.select-actions {
  gap: var(--app-spacing-sm);
}

.selection-count {
  display: inline-flex !important;
  align-items: center;
  height: 28px;
  padding-right: 14px;
  margin-right: 2px;
  border-right: 1px solid var(--md-sys-color-outline-variant);
  color: var(--md-sys-color-primary) !important;
  font-size: 13px !important;
  font-weight: 600;
  white-space: nowrap;
}

.empty-state {
  min-height: 360px;
}

.stash-content {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-md);
}

.stash-list {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-sm);
}

.stash-entry {
  display: grid;
  grid-template-columns: 38px minmax(0, 1fr) auto;
  gap: var(--app-spacing-md);
  align-items: center;
  padding: 15px 16px;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
  background: var(--el-fill-color-blank);
}

.stash-index {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-primary);
  font-weight: 700;
}

.stash-entry-main {
  min-width: 0;
}

.stash-entry-heading {
  gap: var(--app-spacing-sm);
}

.stash-entry-heading strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stash-entry-meta {
  gap: var(--app-spacing-md);
  margin-top: 5px;
}

.stash-entry-meta span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.stash-path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.create-dialog {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-md);
}

.create-toolbar {
  gap: var(--app-spacing-md);
}

.create-toolbar strong,
.create-toolbar span {
  display: block;
}

.diff-file-list {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-md);
  max-height: 52vh;
  overflow: auto;
  padding: 0 8px 6px 0;
  scrollbar-gutter: stable;
}

.diff-file-list > section {
  flex: 0 0 auto;
}

.section-heading {
  display: flex;
  align-items: center;
}

.section-heading {
  justify-content: space-between;
  gap: var(--app-spacing-md);
  padding: 12px 14px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  background: var(--el-fill-color-light);
}

.section-heading strong,
.section-heading span {
  display: block;
}

.section-heading span {
  margin-top: 2px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}

.unversioned-section {
  overflow: hidden;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
}

.unversioned-file {
  border-top: 1px solid var(--md-sys-color-outline-variant);
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.unversioned-file-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
}

.unversioned-file-path {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
}

.unversioned-preview {
  border-top: 1px solid var(--md-sys-color-outline-variant);
  background: var(--el-bg-color);
  cursor: default;
}

.unversioned-preview-path,
.unversioned-preview-note {
  padding: 7px 14px;
  color: var(--el-text-color-secondary);
  font: 12px/1.5 "Cascadia Mono", Consolas, Monaco, monospace;
}

.unversioned-preview-path {
  border-top: 1px solid var(--el-border-color-lighter);
  background: var(--el-fill-color-lighter);
  color: var(--el-text-color-regular);
}

.unversioned-lines {
  max-height: 240px;
  padding: 8px 14px 10px 38px;
}

.unversioned-file.selected {
  background: color-mix(in srgb, var(--md-sys-color-primary-container) 34%, transparent);
}

.diff-file {
  overflow: hidden;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
}

.diff-file-header {
  padding: 10px 12px;
  background: var(--el-fill-color-light);
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.diff-file-header.selected {
  background: color-mix(in srgb, var(--md-sys-color-primary-container) 42%, var(--el-fill-color-light));
}

.diff-file-path {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
}

.hunk-card {
  position: relative;
  border-top: 1px solid var(--md-sys-color-outline-variant);
  cursor: pointer;
  transition: background-color 0.15s ease, box-shadow 0.15s ease;
}

.hunk-card.selected {
  background: color-mix(in srgb, var(--md-sys-color-primary-container) 34%, transparent);
  box-shadow: inset 4px 0 0 var(--md-sys-color-primary);
}

.hunk-card:hover:not(.selected),
.unversioned-file:hover:not(.selected) {
  background: var(--el-fill-color-lighter);
}

.hunk-heading {
  justify-content: space-between;
  gap: var(--app-spacing-sm);
  padding: 8px 12px;
}

.hunk-stats {
  gap: var(--app-spacing-sm);
  font-family: monospace;
  font-size: 12px;
}

.added,
.line-added { color: var(--md-sys-color-success); }
.removed,
.line-removed { color: var(--md-sys-color-error); }

.hunk-lines {
  margin: 0;
  padding: 0 12px 10px 42px;
  overflow: auto;
  color: var(--el-text-color-regular);
  font: 12px/1.65 "Cascadia Mono", Consolas, Monaco, monospace;
  white-space: pre;
}

.hunk-lines span {
  display: block;
  min-height: 20px;
}

.line-meta { color: var(--md-sys-color-secondary); }

.stash-name-field {
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding-top: 2px;
}

.stash-name-field label {
  color: var(--el-text-color-primary);
  font-weight: 600;
  font-size: 13px;
}

.stash-name-field :deep(.el-input__wrapper) {
  border-radius: var(--app-radius-md);
}

@media (max-width: 760px) {
  .stash-entry {
    grid-template-columns: 30px minmax(0, 1fr);
  }

  .stash-entry-actions {
    grid-column: 2;
  }

  .card-subtitle,
  .stash-path {
    display: none !important;
  }
}
</style>
