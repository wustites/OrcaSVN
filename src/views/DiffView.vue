<template>
  <div class="diff-view">
    <el-card class="diff-card">
      <template #header>
        <div class="card-header">
          <span>{{ $t('diff.title') }}</span>
          <div class="header-actions">
            <el-input
              v-model="currentPath"
              :placeholder="$t('diff.filePath')"
              clearable
              style="width: 300px"
            >
              <template #prepend>
                <el-icon><Document /></el-icon>
              </template>
            </el-input>
            <el-button @click="loadDiff" :loading="loading">
              <el-icon><Connection /></el-icon>
              {{ $t('diff.compare') }}
            </el-button>
          </div>
        </div>
      </template>

      <div class="diff-options">
        <el-radio-group v-model="diffType">
          <el-radio label="working">{{ $t('diff.workingCopy') }}</el-radio>
          <el-radio label="revision">{{ $t('diff.betweenRevisions') }}</el-radio>
        </el-radio-group>

        <div v-if="diffType === 'revision'" class="revision-inputs">
          <el-input
            v-model.number="oldRev"
            type="number"
            :placeholder="$t('diff.oldRevision')"
            style="width: 120px"
          />
          <span>{{ $t('diff.to') }}</span>
          <el-input
            v-model.number="newRev"
            type="number"
            :placeholder="$t('diff.newRevision')"
            style="width: 120px"
          />
        </div>
      </div>

      <div v-if="!diffResult && !loading" class="empty-diff">
        <el-empty :description="$t('diff.selectFileAndCompare')" />
      </div>

      <div v-else-if="loading" class="loading-diff">
        <el-skeleton :rows="20" animated />
      </div>

      <div v-else class="diff-content">
        <div class="diff-header">
          <el-tag>{{ $t('diff.file') }}：{{ diffResult?.path }}</el-tag>
          <el-tag v-if="diffResult?.old_revision && diffResult?.new_revision">
            {{ $t('common.version') }}：{{ diffResult?.old_revision }} → {{ diffResult?.new_revision }}
          </el-tag>
        </div>

        <div class="diff-lines" v-html="formattedDiff"></div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { svnDiff } from '@/api/svn'
import type { DiffResult } from '@/types'

const route = useRoute()

const currentPath = ref('')
const diffType = ref<'working' | 'revision'>('working')
const oldRev = ref<number>()
const newRev = ref<number>()
const loading = ref(false)
const diffResult = ref<DiffResult | null>(null)

const formattedDiff = computed(() => {
  if (!diffResult.value?.diff) return ''

  const lines = diffResult.value.diff.split('\n')
  return lines.map(line => {
    if (line.startsWith('+') && !line.startsWith('+++')) {
      return `<div class="diff-line diff-added">${escapeHtml(line)}</div>`
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      return `<div class="diff-line diff-removed">${escapeHtml(line)}</div>`
    } else {
      return `<div class="diff-line">${escapeHtml(line)}</div>`
    }
  }).join('')
})

const escapeHtml = (text: string): string => {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

const loadDiff = async () => {
  const path = currentPath.value || route.query.path as string
  if (!path) return

  loading.value = true
  diffResult.value = null

  try {
    const old = diffType.value === 'revision' ? oldRev.value : undefined
    const new_ = diffType.value === 'revision' ? newRev.value : undefined

    diffResult.value = await svnDiff(path, old, new_)
  } catch (err) {
    console.error('对比失败:', err)
  } finally {
    loading.value = false
  }
}

watch(() => route.query.path, (path) => {
  if (path) {
    currentPath.value = path as string
    loadDiff()
  }
}, { immediate: true })
</script>

<style scoped>
.diff-view {
  height: 100%;
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

.diff-options {
  margin-bottom: 20px;
  display: flex;
  align-items: center;
  gap: 20px;
}

.revision-inputs {
  display: flex;
  align-items: center;
  gap: 10px;
}

.empty-diff,
.loading-diff {
  padding: 40px 0;
}

.diff-content {
  margin-top: 20px;
}

.diff-header {
  display: flex;
  gap: 10px;
  margin-bottom: 15px;
}

.diff-lines {
  background: #f6f8fa;
  border: 1px solid #e1e4e8;
  border-radius: 4px;
  max-height: 600px;
  overflow: auto;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
}

.diff-line {
  padding: 2px 8px;
  white-space: pre;
}

.diff-added {
  background-color: #e6ffed;
}

.diff-removed {
  background-color: #ffeef0;
}
</style>
