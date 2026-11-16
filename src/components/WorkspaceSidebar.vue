<template>
  <aside class="workspace-panel" :class="{ collapsed: store.workspaceListCollapsed }" :aria-label="t('workspacePanel.title')">
    <header class="panel-heading">
      <button class="collapse-toggle" :aria-expanded="!store.workspaceListCollapsed" :title="t(store.workspaceListCollapsed ? 'workspacePanel.expand' : 'workspacePanel.collapse')" @click="toggleCollapsed">
        <el-icon><ArrowRight v-if="store.workspaceListCollapsed" /><ArrowLeft v-else /></el-icon>
        <h2 v-if="!store.workspaceListCollapsed">{{ t('workspacePanel.title') }} <span>{{ store.recentWorkspaces.length }}</span></h2>
      </button>
      <button v-if="!store.workspaceListCollapsed" class="icon-button" :disabled="refreshing" :title="t('workspacePanel.refresh')" :aria-label="t('workspacePanel.refresh')" @click="refreshSummaries">
        <el-icon :class="{ spinning: refreshing }"><Refresh /></el-icon>
      </button>
    </header>
    <button v-show="!store.workspaceListCollapsed" class="add-workspace" @click="emit('open')"><el-icon><Plus /></el-icon>{{ t('workspace.addWorkspace') }}</button>
    <div v-show="!store.workspaceListCollapsed" class="workspace-tabs" role="tablist" :aria-label="t('workspacePanel.title')">
      <button id="workspaces-tab-all" type="button" role="tab" :aria-selected="activeTab === 'all'" aria-controls="workspaces-tab-panel" :tabindex="activeTab === 'all' ? 0 : -1" @click="changeTab('all')" @keydown="navigateTabs($event)">
        {{ t('workspacePanel.tabAll') }} <span>{{ store.recentWorkspaces.length }}</span>
      </button>
      <button id="workspaces-tab-favorites" type="button" role="tab" :aria-selected="activeTab === 'favorites'" aria-controls="workspaces-tab-panel" :tabindex="activeTab === 'favorites' ? 0 : -1" @click="changeTab('favorites')" @keydown="navigateTabs($event)">
        {{ t('workspacePanel.tabFavorites') }} <span>{{ store.favoriteWorkspaces.length }}</span>
      </button>
    </div>
    <div v-show="!store.workspaceListCollapsed" class="sort-toolbar">
      <label for="workspace-sort">{{ t('workspacePanel.sortLabel') }}</label>
      <select id="workspace-sort" :value="store.workspaceSortMode" @change="changeSort">
        <option value="manual">{{ t('workspacePanel.sortManual') }}</option>
        <option value="name">{{ t('workspacePanel.sortName') }}</option>
        <option value="modified">{{ t('workspacePanel.sortModified') }}</option>
      </select>
    </div>
    <p v-show="!store.workspaceListCollapsed" class="drag-help">{{ t('workspacePanel.dragHelp') }}</p>
    <div id="workspaces-tab-panel" v-show="!store.workspaceListCollapsed" class="workspace-list" role="tabpanel" :aria-labelledby="activeTab === 'all' ? 'workspaces-tab-all' : 'workspaces-tab-favorites'" :aria-busy="refreshing" @dragover="scrollWhileDragging" @drop.prevent="endDrag">
      <p v-if="!visibleWorkspaces.length" class="empty-list">{{ t(activeTab === 'favorites' ? 'workspacePanel.emptyFavorites' : 'workspacePanel.empty') }}</p>
      <article v-for="path in visibleWorkspaces" :key="path" class="workspace-card" :class="{ selected: path === store.currentPath, dragging: dragPath === path, 'drop-before': dropPath === path && !dropAfter, 'drop-after': dropPath === path && dropAfter }"
        draggable="true" @dragstart="startDrag($event, path)" @dragover.prevent="dragOver($event, path)" @drop.prevent.stop="dropWorkspace(path)" @dragend="endDrag">
        <button class="workspace-select" :aria-pressed="path === store.currentPath" :title="workspaceTitle(path)" @click="emit('select', path)">
          <span class="workspace-name"><strong>{{ workspaceName(path) }}</strong><span v-if="store.workspaceNotes[path]">:{{ store.workspaceNotes[path] }}</span></span>
          <span class="workspace-counts">
            <span :title="t('workspacePanel.changedHelp')">{{ t('workspacePanel.changed') }} <b>{{ store.workspaceSummaries[path]?.changed ?? '—' }}</b></span>
            <span :class="{ missing: store.workspaceSummaries[path]?.missing }">{{ t('status.missing') }} <b>{{ store.workspaceSummaries[path]?.missing ?? '—' }}</b></span>
            <time :title="t('workspacePanel.modifiedHelp')">{{ formatTime(store.workspaceSummaries[path]?.latestModified) }}</time>
          </span>
        </button>
        <button type="button" class="icon-button workspace-favorite" :class="{ 'is-favorite': store.isWorkspaceFavorite(path) }" :aria-pressed="store.isWorkspaceFavorite(path)" :title="favoriteLabel(path)" :aria-label="favoriteLabel(path) + ': ' + workspaceName(path)" draggable="false" @dragstart.prevent.stop @click.stop="toggleFavorite(path)">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m12 3 2.8 5.7 6.3.9-4.55 4.45 1.07 6.28L12 17.36l-5.62 2.97 1.07-6.28L2.9 9.6l6.3-.9Z" /></svg>
        </button>
        <div class="workspace-actions">
          <WorkspaceOpenMenu :path="path" compact />
          <button class="icon-button drag-handle" :title="t('workspacePanel.dragHandle')" :aria-label="t('workspacePanel.dragHandle')" @keydown.up.prevent="moveByKey(path, -1)" @keydown.down.prevent="moveByKey(path, 1)">⠿</button>
          <span v-if="summaryError(path)" class="summary-error" role="img" :title="summaryError(path)" :aria-label="t('workspacePanel.scanFailed')">!</span>
          <button class="icon-button workspace-note" :title="t('workspacePanel.editNote')" :aria-label="t('workspacePanel.editNote')" @click="editNote(path)"><el-icon><Edit /></el-icon></button>
          <button class="icon-button remove-workspace" :title="t('workspacePanel.removeHelp')" :aria-label="t('workspace.removeWorkspace') + ': ' + workspaceName(path)" @click="removeWorkspace(path)"><el-icon><Close /></el-icon></button>
        </div>
      </article>
    </div>
    <el-dialog v-model="noteVisible" :title="t('workspacePanel.editNote')" width="min(460px, 90vw)" append-to-body>
      <p class="note-path">{{ workspaceName(notePath) }}</p>
      <el-input v-model="noteDraft" type="textarea" :rows="4" :maxlength="200" show-word-limit :aria-label="t('workspacePanel.editNote')" :placeholder="t('workspacePanel.notePlaceholder')" />
      <template #footer>
        <el-button @click="noteVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveNote">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>
  </aside>
</template>

<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import { useWorkspaceStore, type WorkspaceSortMode } from '@/stores/workspace'
import { useWorkspaceSummaries } from '@/composables/useWorkspaceSummaries'
import WorkspaceOpenMenu from '@/components/WorkspaceOpenMenu.vue'

const emit = defineEmits<{ open: []; select: [path: string] }>()
const { t } = useI18n()
const store = useWorkspaceStore()
const { refreshSummaries, refreshing } = useWorkspaceSummaries()
const activeTab = ref<'all' | 'favorites'>('all')
const visibleWorkspaces = computed(() => activeTab.value === 'favorites' ? store.sortedFavoriteWorkspaces : store.sortedWorkspaces)
const noteVisible = ref(false)
const notePath = ref('')
const noteDraft = ref('')
const workspaceName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() || path
const workspaceTitle = (path: string) => workspaceName(path) + (store.workspaceNotes[path] ? ':' + store.workspaceNotes[path] : '')
const summaryError = (path: string) => store.workspaceSummaryErrors[path] || store.workspaceSummaries[path]?.error

const dragPath = ref<string | null>(null)
const dropPath = ref<string | null>(null)
const dropAfter = ref(false)
let scrollFrame = 0
let scrollTarget: HTMLElement | null = null
let scrollStep = 0

function reportAction(action: () => void) {
  try { action() } catch (error) { ElMessage.error(String(error)) }
}
function changeTab(tab: 'all' | 'favorites') {
  endDrag()
  activeTab.value = tab
}
function navigateTabs(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
  event.preventDefault()
  changeTab(event.key === 'Home' ? 'all' : event.key === 'End' ? 'favorites' : activeTab.value === 'all' ? 'favorites' : 'all')
  document.getElementById(activeTab.value === 'all' ? 'workspaces-tab-all' : 'workspaces-tab-favorites')?.focus()
}
function favoriteLabel(path: string) {
  return t(store.isWorkspaceFavorite(path) ? 'workspacePanel.removeFavorite' : 'workspacePanel.addFavorite')
}
function toggleFavorite(path: string) {
  endDrag()
  reportAction(() => store.setWorkspaceFavorite(path, !store.isWorkspaceFavorite(path)))
}
function toggleCollapsed() {
  endDrag()
  reportAction(() => store.setWorkspaceListCollapsed(!store.workspaceListCollapsed))
}
function changeSort(event: Event) {
  const select = event.target as HTMLSelectElement
  reportAction(() => store.setWorkspaceSortMode(select.value as WorkspaceSortMode))
  select.value = store.workspaceSortMode
}
function removeWorkspace(path: string) {
  reportAction(() => store.removeRecentWorkspace(path))
}
function startDrag(event: DragEvent, path: string) {
  if (event.target instanceof Element && (event.target.closest('.workspace-favorite') || (event.target.closest('.workspace-actions') && !event.target.closest('.drag-handle')))) {
    event.preventDefault()
    return
  }
  dragPath.value = path
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', path)
  }
}
function dragOver(event: DragEvent, path: string) {
  if (!dragPath.value || dragPath.value === path) { dropPath.value = null; return }
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  dropPath.value = path
  dropAfter.value = event.clientY > rect.top + rect.height / 2
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move'
}
function dropWorkspace(path: string) {
  if (dragPath.value && dragPath.value !== path) {
    const source = dragPath.value
    reportAction(() => store.reorderWorkspaces(source, path, dropAfter.value))
  }
  endDrag()
}
function moveByKey(path: string, direction: number) {
  const target = visibleWorkspaces.value[visibleWorkspaces.value.indexOf(path) + direction]
  if (target) reportAction(() => store.reorderWorkspaces(path, target, direction > 0))
}
function scrollWhileDragging(event: DragEvent) {
  if (!dragPath.value) return
  event.preventDefault()
  scrollTarget = event.currentTarget as HTMLElement
  const rect = scrollTarget.getBoundingClientRect()
  scrollStep = event.clientY < rect.top + 36 ? -7 : event.clientY > rect.bottom - 36 ? 7 : 0
  if (!scrollFrame && scrollStep) scrollFrame = requestAnimationFrame(scrollList)
}
function scrollList() {
  scrollFrame = 0
  if (!dragPath.value || !scrollTarget || !scrollStep) return
  scrollTarget.scrollTop += scrollStep
  scrollFrame = requestAnimationFrame(scrollList)
}
function endDrag() {
  dragPath.value = null
  dropPath.value = null
  scrollStep = 0
  cancelAnimationFrame(scrollFrame)
  scrollFrame = 0
}
onBeforeUnmount(endDrag)

function formatTime(value?: number | null) {
  if (!value) return '—'
  const date = new Date(value)
  const pad = (part: number) => String(part).padStart(2, '0')
  return `${date.getFullYear()}/${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`
}

function editNote(path: string) {
  notePath.value = path
  noteDraft.value = store.workspaceNotes[path] || ''
  noteVisible.value = true
}

function saveNote() {
  try {
    store.setWorkspaceNote(notePath.value, noteDraft.value)
    noteVisible.value = false
  } catch (error) {
    ElMessage.error(String(error))
  }
}

onMounted(() => { void refreshSummaries() })
</script>

<style scoped>
.workspace-panel { display: flex; flex-direction: column; min-width: 0; min-height: 0; overflow: hidden; border-right: 1px solid var(--md-sys-color-outline-variant); background: var(--md-sys-color-surface-container-lowest); }
.panel-heading { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px 4px; }
.panel-heading h2 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 13px; }
.panel-heading h2 span { color: var(--md-sys-color-on-surface-variant); font-size: 11px; font-weight: 400; }
button { font: inherit; cursor: pointer; }
button:focus-visible { outline: 2px solid var(--md-sys-color-primary); outline-offset: -2px; }
.icon-button { display: grid; place-items: center; width: 24px; height: 24px; padding: 0; border: 0; border-radius: 4px; color: var(--md-sys-color-on-surface-variant); background: transparent; }
.icon-button:hover { background: var(--md-sys-state-hover); }
.icon-button:disabled { opacity: .5; cursor: wait; }
.add-workspace { display: flex; flex-shrink: 0; justify-content: center; align-items: center; gap: 6px; margin: 2px 8px 8px; padding: 5px; border: 1px dashed var(--md-sys-color-outline-variant); border-radius: 5px; background: transparent; color: var(--md-sys-color-primary); font-size: 12px; }
.add-workspace:hover { background: var(--md-sys-color-surface-container-low); }
.workspace-tabs { display: flex; flex-shrink: 0; gap: 4px; margin: 0 8px 8px; padding: 3px; border-radius: 6px; background: var(--md-sys-color-surface-container); }
.workspace-tabs button { display: flex; flex: 1; justify-content: center; align-items: center; gap: 6px; padding: 5px 6px; border: 0; border-radius: 4px; background: transparent; color: var(--md-sys-color-on-surface-variant); font-size: 12px; }
.workspace-tabs button[aria-selected="true"] { background: var(--md-sys-color-surface-container-lowest); color: var(--md-sys-color-primary); font-weight: 600; }
.workspace-tabs span { font-size: 10px; font-weight: 400; font-variant-numeric: tabular-nums; }
.workspace-list { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; padding: 0 6px 6px; scrollbar-gutter: stable; }
.empty-list { padding: 12px 6px; color: var(--md-sys-color-on-surface-variant); font-size: 12px; line-height: 1.6; }
.workspace-card { position: relative; margin-bottom: 4px; border: 1px solid var(--md-sys-color-outline-variant); border-radius: 5px; background: var(--md-sys-color-surface-container-lowest); overflow: hidden; }
.workspace-card.selected { border-color: var(--md-sys-color-primary); background: var(--md-sys-color-surface-container-low); box-shadow: inset 3px 0 var(--md-sys-color-primary); }
.workspace-select { display: flex; flex-direction: column; gap: 4px; width: 100%; padding: 7px 8px; border: 0; text-align: left; background: transparent; color: var(--md-sys-color-on-surface); }
.workspace-select:hover { background: var(--md-sys-state-hover); }
.workspace-name { display: block; width: calc(100% - 24px); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; line-height: 18px; }
.workspace-name strong { font-weight: 600; }
.workspace-counts { display: flex; align-items: center; gap: 9px; white-space: nowrap; font-size: 11px; line-height: 16px; font-variant-numeric: tabular-nums; }
.workspace-counts b { margin-left: 2px; font-weight: 600; }
.workspace-counts .missing, .summary-error { color: var(--md-sys-color-error); }
.workspace-counts time { color: var(--md-sys-color-on-surface-variant); }
.workspace-favorite { position: absolute; top: 3px; right: 3px; }
.workspace-favorite svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linejoin: round; }
.workspace-favorite.is-favorite { color: var(--md-sys-color-primary); }
.workspace-favorite.is-favorite svg { fill: currentColor; }
.workspace-actions { position: absolute; top: 3px; right: 27px; display: flex; align-items: center; border-radius: 4px; background: var(--md-sys-color-surface-container-lowest); opacity: 0; pointer-events: none; }
.workspace-card:hover .workspace-actions, .workspace-card:focus-within .workspace-actions { opacity: 1; pointer-events: auto; }
.workspace-card:has(.summary-error) .workspace-actions { opacity: 1; pointer-events: auto; }
.summary-error { display: grid; place-items: center; width: 18px; height: 24px; font-size: 12px; font-weight: 700; }
.note-path { overflow-wrap: anywhere; color: var(--md-sys-color-on-surface-variant); font-size: 12px; }
.spinning { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (hover: none) { .workspace-actions { opacity: 1; pointer-events: auto; } .workspace-name { width: calc(100% - 48px); } }
.collapse-toggle { display: flex; align-items: center; gap: 5px; min-width: 0; border: 0; padding: 0; background: transparent; color: inherit; }
.collapse-toggle .el-icon { width: 24px; height: 24px; }
.collapsed .panel-heading { padding: 10px; justify-content: center; }
.sort-toolbar { display: flex; align-items: center; gap: 6px; padding: 0 8px; font-size: 11px; }
.sort-toolbar label { flex-shrink: 0; color: var(--md-sys-color-on-surface-variant); }
.sort-toolbar select { flex: 1; min-width: 0; padding: 4px; border: 1px solid var(--md-sys-color-outline-variant); border-radius: 4px; background: var(--md-sys-color-surface-container-lowest); color: inherit; font: inherit; }
.drag-help { margin: 5px 8px 7px; color: var(--md-sys-color-on-surface-variant); font-size: 10px; line-height: 1.4; }
.drag-handle { cursor: grab; }
.workspace-card.dragging { opacity: .45; }
.workspace-card.drop-before { border-top: 3px solid var(--md-sys-color-primary); }
.workspace-card.drop-after { border-bottom: 3px solid var(--md-sys-color-primary); }
</style>
