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
            <el-button type="primary" size="small" @click="openCreateDialog" :disabled="!hasChanges">
              <el-icon><Plus /></el-icon>
              {{ $t('stash.create') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-empty v-if="!workspaceStore.currentPath" :description="$t('log.openWorkspaceFirst')" class="empty-state" />

      <div v-else class="stash-content">
        <el-alert
          v-if="hasChanges"
          :title="$t('stash.workingCopyHint', { count: changedFiles.length })"
          type="info"
          :closable="false"
          show-icon
        />

        <el-empty v-if="stashes.length === 0" :description="$t('stash.empty')" class="empty-state">
          <el-button type="primary" :disabled="!hasChanges" @click="openCreateDialog">
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
                <span class="stash-path" :title="entry.workspacePath">{{ entry.workspacePath }}</span>
              </div>
            </div>
            <div class="stash-entry-actions">
              <el-button size="small" @click="applyStash(entry)">{{ $t('stash.apply') }}</el-button>
              <el-button size="small" type="primary" @click="popStash(entry)">{{ $t('stash.pop') }}</el-button>
              <el-dropdown trigger="click" @command="handleEntryDropdown(entry)">
                <el-button size="small" text>
                  <el-icon><MoreFilled /></el-icon>
                </el-button>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="copy"><el-icon><CopyDocument /></el-icon>{{ $t('stash.copy') }}</el-dropdown-item>
                    <el-dropdown-item command="export"><el-icon><Download /></el-icon>{{ $t('stash.export') }}</el-dropdown-item>
                    <el-dropdown-item divided command="delete"><el-icon><Delete /></el-icon>{{ $t('common.delete') }}</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </article>
        </div>
      </div>
    </el-card>

    <el-dialog v-model="createDialogVisible" :title="$t('stash.createTitle')" width="min(1000px, 92vw)" top="5vh" destroy-on-close>
      <div class="create-dialog">
        <div class="create-toolbar">
          <div>
            <strong>{{ $t('stash.selectHunks') }}</strong>
            <span>{{ $t('stash.selectHunksHint') }}</span>
          </div>
          <div class="select-actions">
            <el-button text size="small" @click="selectAllHunks">{{ $t('stash.selectAll') }}</el-button>
            <el-button text size="small" @click="clearSelectedHunks">{{ $t('stash.clearSelection') }}</el-button>
          </div>
        </div>

        <el-skeleton v-if="loadingDiffs" :rows="8" animated />
        <el-empty v-else-if="diffFiles.length === 0" :description="$t('stash.noDiffs')" />
        <div v-else class="diff-file-list">
          <section v-for="file in diffFiles" :key="file.path" class="diff-file">
            <div class="diff-file-header">
              <el-checkbox
                :model-value="file.hunks.every(hunk => selectedHunks.includes(hunk.id))"
                :indeterminate="file.hunks.some(hunk => selectedHunks.includes(hunk.id)) && !file.hunks.every(hunk => selectedHunks.includes(hunk.id))"
                @change="onFileToggle(file, $event)"
              >
                <span class="diff-file-path">{{ file.path }}</span>
              </el-checkbox>
              <span class="hunk-count">{{ $t('stash.hunks', { count: file.hunks.length }) }}</span>
            </div>
            <div v-for="hunk in file.hunks" :key="hunk.id" class="hunk-card" :class="{ selected: selectedHunks.includes(hunk.id) }">
              <div class="hunk-heading">
                <el-checkbox :model-value="selectedHunks.includes(hunk.id)" @change="onHunkToggle(hunk.id, $event)">
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

        <el-input v-model="stashName" :label="$t('stash.name')" :placeholder="$t('stash.namePlaceholder')" class="stash-name-input">
          <template #prepend>{{ $t('stash.name') }}</template>
        </el-input>
      </div>
      <template #footer>
        <el-button @click="createDialogVisible = false">{{ $t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="creating" :disabled="selectedHunks.length === 0" @click="createStash">
          {{ $t('stash.saveAndHide') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { svnApplyPatch, svnDiff } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useStashStore } from '@/stores/stash'
import { useWorkspace } from '@/composables/useWorkspace'
import { buildPatch, createStashId, parseUnifiedDiff } from '@/utils/stash'
import type { StashEntry, StashFile } from '@/types'
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
const stashName = ref('')

const changedFiles = computed(() => workspaceStore.statusList.filter(status => status.status_code !== 'normal' && status.status_code !== ''))
const hasChanges = computed(() => changedFiles.value.length > 0)
const stashes = computed(() => stashStore.currentEntries(workspaceStore.currentPath))

const formatDate = (date: string) => new Date(date).toLocaleString(locale.value)

const lineClass = (line: string) => ({
  'line-added': line.startsWith('+') && !line.startsWith('+++'),
  'line-removed': line.startsWith('-') && !line.startsWith('---'),
  'line-meta': line.startsWith('\\'),
})

const openCreateDialog = async () => {
  if (!workspaceStore.currentPath) return
  createDialogVisible.value = true
  stashName.value = `WIP on ${new Date().toLocaleString(locale.value)}`
  selectedHunks.value = []
  loadingDiffs.value = true
  try {
    const files = changedFiles.value.filter(file => file.status_code !== 'unversioned')
    const parsed = await Promise.all(files.map(async file => {
      const result = await svnDiff(workspaceStore.currentPath!, file.path)
      return parseUnifiedDiff(file.path, result.diff)
    }))
    diffFiles.value = parsed.filter((file): file is StashFile => file !== null)
    selectAllHunks()
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
}

const clearSelectedHunks = () => { selectedHunks.value = [] }

const createStash = async () => {
  if (!workspaceStore.currentPath || selectedHunks.value.length === 0) return
  const patch = buildPatch(diffFiles.value, new Set(selectedHunks.value))
  if (!patch) {
    ElMessage.warning(t('stash.noSelection'))
    return
  }

  creating.value = true
  try {
    await svnApplyPatch(workspaceStore.currentPath, patch, true)
    stashStore.addEntry({
      id: createStashId(),
      name: stashName.value.trim() || t('stash.defaultName'),
      workspacePath: workspaceStore.currentPath,
      createdAt: new Date().toISOString(),
      patch,
      files: [...new Set(diffFiles.value.filter(file => file.hunks.some(hunk => selectedHunks.value.includes(hunk.id))).map(file => file.path))],
      hunkCount: selectedHunks.value.length,
    })
    createDialogVisible.value = false
    await refreshStatus()
    ElMessage.success(t('stash.created'))
  } catch (err) {
    ElMessage.error(`${t('stash.createFailed')}：${err}`)
  } finally {
    creating.value = false
  }
}

const applyEntry = async (entry: StashEntry, removeAfterApply: boolean) => {
  if (!workspaceStore.currentPath) return
  try {
    await svnApplyPatch(workspaceStore.currentPath, entry.patch)
    if (removeAfterApply) stashStore.removeEntry(entry.id)
    await refreshStatus()
    ElMessage.success(removeAfterApply ? t('stash.popped') : t('stash.applied'))
  } catch (err) {
    ElMessage.error(`${t('stash.applyFailed')}：${err}`)
  }
}

const applyStash = (entry: StashEntry) => applyEntry(entry, false)
const popStash = (entry: StashEntry) => applyEntry(entry, true)

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
  } catch {
    // Cancelled.
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
  padding-right: 4px;
}

.diff-file {
  overflow: hidden;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
}

.diff-file-header {
  padding: 10px 12px;
  background: var(--el-fill-color-light);
}

.diff-file-path {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 13px;
}

.hunk-card {
  border-top: 1px solid var(--md-sys-color-outline-variant);
}

.hunk-card.selected {
  background: color-mix(in srgb, var(--md-sys-color-primary-container) 22%, transparent);
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

.stash-name-input {
  max-width: 560px;
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
