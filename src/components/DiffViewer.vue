<template>
  <section class="diff-viewer" :class="{ compact }" @keydown="handleFindShortcut">
    <div class="diff-tools">
      <input ref="searchInput" v-model="query" class="diff-search" type="search"
        :placeholder="$t('diff.findText')" :aria-label="$t('diff.findText')"
        :disabled="indexing" @keydown.enter.prevent="find($event.shiftKey ? 'previous' : 'next')"
        @keydown.esc.prevent="query = ''" />
      <span v-if="noMatch" class="search-status" role="status">{{ $t('diff.noMatch') }}</span>
      <button type="button" :disabled="!query || indexing" :title="$t('diff.previousMatch')"
        :aria-label="$t('diff.previousMatch')" @click="find('previous')"><el-icon><ArrowLeft /></el-icon></button>
      <button type="button" :disabled="!query || indexing" :title="$t('diff.nextMatch')"
        :aria-label="$t('diff.nextMatch')" @click="find('next')"><el-icon><ArrowRight /></el-icon></button>
      <button type="button" :disabled="!text || copying" :title="$t('diff.copyAll')"
        :aria-label="$t('diff.copyAll')" @click="copyAll"><el-icon><CopyDocument /></el-icon></button>
    </div>
    <div v-if="indexing" class="indexing-diff" role="status">{{ $t('diff.preparing') }}</div>
    <VirtualViewport v-else ref="viewport" class="diff-lines" :item-count="documentIndex?.offsets.length || 0"
      :row-height="rowHeight" :content-width="contentWidth" role="table" :aria-label="$t('diff.title')"
      :aria-rowcount="documentIndex?.offsets.length || 0">
      <template #default="{ start, end }">
        <div v-for="line in visibleRows(start, end)" :key="line.index" class="diff-row"
          :class="[line.className, { 'is-search-match': matchedLine === line.index - 1 }]"
          :style="{ height: `${rowHeight}px` }" role="row" :aria-rowindex="line.index">
          <span class="diff-line-number" role="cell">{{ line.index }}</span>
          <span class="diff-marker" role="cell">{{ line.marker }}</span>
          <code class="diff-code" role="cell"><template v-for="(part, partIndex) in highlight(line)" :key="partIndex"><mark v-if="part.match">{{ part.text }}</mark><template v-else>{{ part.text }}</template></template></code>
        </div>
      </template>
    </VirtualViewport>
  </section>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import VirtualViewport from './VirtualViewport.vue'
import { findDiffMatch, getDiffRows, indexDiff, lineAtOffset, type DiffIndex, type DiffRow } from '@/utils/diffDocument'
import { writeClipboardText } from '@/utils/clipboard'

const props = withDefaults(defineProps<{ text: string; compact?: boolean }>(), { compact: false })
const emit = defineEmits<{ stats: [stats: { added: number; removed: number }] }>()
const { t } = useI18n()
const viewport = ref<InstanceType<typeof VirtualViewport> | null>(null)
const searchInput = ref<HTMLInputElement | null>(null)
const documentIndex = shallowRef<DiffIndex | null>(null)
const indexing = ref(false)
const query = ref('')
const activeOffset = ref(-1)
const noMatch = ref(false)
const copying = ref(false)
const characterWidth = ref(8)
const rowHeight = computed(() => props.compact ? 24 : 28)
const gutterWidth = computed(() => props.compact ? 70 : 98)
const contentWidth = computed(() => gutterWidth.value + 24 + (documentIndex.value?.maxColumns || 0) * characterWidth.value)
const matchedLine = computed(() => activeOffset.value >= 0 && documentIndex.value ? lineAtOffset(documentIndex.value, activeOffset.value) : -1)
let worker: Worker | undefined
let searchTimer: ReturnType<typeof setTimeout> | undefined
let searchGeneration = 0
let documentGeneration = 0
let disposed = false

function setIndex(index: DiffIndex) {
  documentIndex.value = index
  indexing.value = false
  emit('stats', { added: index.added, removed: index.removed })
}
async function applyMatch(offset: number, generation: number) {
  if (generation !== searchGeneration || disposed) return
  activeOffset.value = offset
  noMatch.value = offset < 0 && query.value.length > 0
  await nextTick()
  if (generation === searchGeneration && offset >= 0) viewport.value?.scrollToIndex(matchedLine.value, 'center')
}
function find(direction: 'next' | 'previous', restart = false) {
  if (indexing.value || !query.value) return
  if (searchTimer !== undefined) clearTimeout(searchTimer)
  searchTimer = undefined
  const id = ++searchGeneration
  const from = restart ? 0 : activeOffset.value < 0
    ? direction === 'next' ? 0 : props.text.length
    : activeOffset.value + (direction === 'next' ? 1 : -1)
  if (worker) worker.postMessage({ type: 'find', id, query: query.value, from, direction })
  else void applyMatch(findDiffMatch(props.text, query.value, from, direction), id)
}
function visibleRows(start: number, end: number) {
  return documentIndex.value ? getDiffRows(props.text, documentIndex.value, start, end) : []
}
function highlight(row: DiffRow) {
  const localOffset = activeOffset.value - row.start
  if (matchedLine.value !== row.index - 1 || localOffset < 0 || !query.value) return [{ text: row.text, match: false }]
  return [
    { text: row.text.slice(0, localOffset), match: false },
    { text: row.text.slice(localOffset, localOffset + query.value.length), match: true },
    { text: row.text.slice(localOffset + query.value.length), match: false },
  ]
}
function handleFindShortcut(event: KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'f') {
    event.preventDefault()
    event.stopPropagation()
    searchInput.value?.focus()
    searchInput.value?.select()
  }
}
async function copyAll() {
  copying.value = true
  try {
    await writeClipboardText(props.text)
    ElMessage.success(t('diff.copied'))
  } catch {
    ElMessage.error(t('diff.copyFailed'))
  } finally {
    copying.value = false
  }
}
watch(() => props.text, text => {
  const generation = ++documentGeneration
  worker?.terminate()
  worker = undefined
  if (searchTimer !== undefined) clearTimeout(searchTimer)
  searchGeneration++
  query.value = ''
  activeOffset.value = -1
  noMatch.value = false
  documentIndex.value = null
  viewport.value?.resetScroll()
  emit('stats', { added: 0, removed: 0 })
  if (text.length < 200_000) {
    setIndex(indexDiff(text))
    return
  }
  indexing.value = true
  try {
    worker = new Worker(new URL('../workers/diff.worker.ts', import.meta.url), { type: 'module' })
    worker.onmessage = (event: MessageEvent<{ type: string; index: DiffIndex; id: number; offset: number }>) => {
      if (disposed || generation !== documentGeneration) return
      if (event.data.type === 'index') setIndex(event.data.index)
      else void applyMatch(event.data.offset, event.data.id)
    }
    worker.onerror = () => {
      if (disposed || generation !== documentGeneration) return
      worker?.terminate()
      worker = undefined
      if (!disposed) setIndex(indexDiff(props.text))
    }
    worker.postMessage({ type: 'index', text })
  } catch {
    worker?.terminate()
    worker = undefined
    setIndex(indexDiff(text))
  }
}, { immediate: true, flush: 'sync' })
watch(query, () => {
  searchGeneration++
  activeOffset.value = -1
  noMatch.value = false
  if (searchTimer !== undefined) clearTimeout(searchTimer)
  if (query.value) searchTimer = setTimeout(() => find('next', true), 150)
})
onMounted(() => {
  const measureFont = () => {
    if (disposed || !viewport.value?.element) return
    const canvas = document.createElement('canvas')
    const context = canvas.getContext('2d')
    if (context) {
      context.font = getComputedStyle(viewport.value.element).font
      characterWidth.value = Math.ceil(context.measureText('M').width) || 8
    }
  }
  measureFont()
  void document.fonts.ready.then(measureFont)
})
onBeforeUnmount(() => {
  disposed = true
  searchGeneration++
  worker?.terminate()
  if (searchTimer !== undefined) clearTimeout(searchTimer)
})
</script>

<style scoped>
.diff-viewer { display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; overflow: hidden; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--app-radius-md); background: var(--md-sys-color-surface-container-lowest); }
.diff-tools { display: flex; align-items: center; flex-shrink: 0; gap: 4px; padding: 5px 8px; border-bottom: 1px solid var(--md-sys-color-outline-variant); background: var(--md-sys-color-surface-container-low); }
.diff-search { flex: 1; width: 0; min-width: 60px; height: 28px; padding: 0 8px; font-size: 12px; border: 1px solid var(--md-sys-color-outline-variant); border-radius: var(--app-radius-sm); background: var(--md-sys-color-surface-container-lowest); }
.diff-tools button { display: grid; place-items: center; flex-shrink: 0; width: 28px; height: 28px; border: 0; border-radius: var(--app-radius-sm); background: transparent; cursor: pointer; }
.diff-tools button:hover:enabled { background: var(--md-sys-state-hover); }
.diff-tools button:disabled { opacity: .4; cursor: default; }
.search-status { font-size: 11px; color: var(--md-sys-color-on-surface-variant); }
.indexing-diff { padding: 16px; color: var(--md-sys-color-on-surface-variant); }
.diff-lines { font-family: var(--app-font-family-mono); font-size: var(--app-font-size-label); line-height: 20px; }
.compact .diff-lines { font-size: 12px; }
.diff-row { display: grid; grid-template-columns: 64px 34px minmax(0, 1fr); border-bottom: 1px solid var(--md-sys-color-outline-variant); }
.compact .diff-row { grid-template-columns: 46px 24px minmax(0, 1fr); }
.diff-line-number, .diff-marker { user-select: none; padding: 3px 8px; color: var(--md-sys-color-on-surface-variant); background: var(--md-sys-color-surface-container-low); text-align: right; }
.diff-marker { text-align: center; font-weight: 700; }
.diff-code { padding: 3px 12px; white-space: pre; tab-size: 8; color: var(--md-sys-color-on-surface); font: inherit; }
.compact .diff-line-number, .compact .diff-marker, .compact .diff-code { padding-top: 1px; padding-bottom: 1px; }
.diff-added { background: var(--app-color-status-added-bg); }
.diff-added .diff-code { color: var(--app-color-status-added-text); }
.diff-added .diff-line-number, .diff-added .diff-marker { background: var(--app-color-status-added-bg); color: var(--app-color-status-added-text); }
.diff-removed { background: var(--app-color-status-error-bg); }
.diff-removed .diff-code { color: var(--app-color-status-error-text); }
.diff-removed .diff-line-number, .diff-removed .diff-marker { background: var(--app-color-status-error-bg); color: var(--app-color-status-error-text); }
.diff-meta { background: var(--app-color-status-unversioned-bg); }
.diff-meta .diff-code, .diff-meta .diff-marker { color: var(--app-color-status-unversioned-text); font-weight: 700; }
.is-search-match { box-shadow: inset 3px 0 var(--md-sys-color-primary); }
mark { background: var(--app-color-status-modified-bg); color: var(--app-color-status-modified-text); }
</style>
