<template>
  <div class="blame-view">
    <el-card class="blame-card">
      <template #header>
        <div class="card-header">
          <span class="view-title">
            <el-icon><Edit /></el-icon>
            {{ $t('blame.title') }}
          </span>
          <div class="header-actions">
            <el-input
              v-model="currentPath"
              :placeholder="$t('diff.filePath')"
              clearable
              class="path-input"
            >
              <template #prepend>
                <el-icon><Document /></el-icon>
              </template>
            </el-input>
            <el-button @click="loadBlame" :loading="loading" type="primary">
              <el-icon><Edit /></el-icon>
              {{ $t('blame.load') }}
            </el-button>
          </div>
        </div>
      </template>

      <div v-if="!blameLines.length && !loading" class="empty-blame">
        <el-empty :description="$t('blame.selectFileToBlame')">
          <template #image>
            <el-icon class="empty-icon"><Edit /></el-icon>
          </template>
        </el-empty>
      </div>

      <div v-else-if="loading" class="loading-blame">
        <el-skeleton :rows="20" animated />
      </div>

      <div v-else class="blame-content">
        <div class="blame-header">
          <div class="blame-tags">
            <el-tag class="path-tag" type="primary">
              <el-icon><Document /></el-icon>
              {{ currentPath }}
            </el-tag>
            <el-tag type="info" effect="plain">
              <el-icon><List /></el-icon>
              {{ blameLines.length }} {{ $t('blame.lines') }}
            </el-tag>
          </div>
        </div>

        <el-table 
          :data="annotatedBlameLines" 
          style="width: 100%" 
          height="100%" 
          row-key="lineNo"
          stripe
          highlight-current-row
          class="blame-table"
        >
          <el-table-column prop="lineNo" label="#" width="72" align="right" fixed>
            <template #default="{ row }">
              <span class="line-number">{{ row.lineNo }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="revision" :label="$t('blame.revision')" width="112" sortable align="center">
            <template #default="{ row }">
              <el-tag type="primary" size="small" effect="plain">r{{ row.revision }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="author" :label="$t('blame.author')" width="150" show-overflow-tooltip>
            <template #default="{ row }">
              <div class="author-cell">
                <el-icon><User /></el-icon>
                <span>{{ row.author }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column prop="line" :label="$t('blame.content')">
            <template #default="{ row }">
              <code class="code-line">{{ row.line }}</code>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { svnBlame } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useI18n } from 'vue-i18n'
import type { BlameLine } from '@/types'

const { t } = useI18n()
const route = useRoute()
const workspaceStore = useWorkspaceStore()

const currentPath = ref('')
const loading = ref(false)
const blameLines = ref<BlameLine[]>([])

const annotatedBlameLines = computed(() => {
  return blameLines.value.map((line, index) => ({
    ...line,
    lineNo: index + 1,
  }))
})

const loadBlame = async () => {
  const file = currentPath.value || route.query.path as string
  if (!file || !workspaceStore.currentPath) return

  loading.value = true
  blameLines.value = []

  try {
    blameLines.value = await svnBlame(workspaceStore.currentPath, file)
  } catch (err) {
    blameLines.value = []
    ElMessage.error(`${t('common.error')}：${err}`)
  } finally {
    loading.value = false
  }
}

watch(() => route.query.path, (path) => {
  if (path) {
    currentPath.value = path as string
    loadBlame()
  }
}, { immediate: true })
</script>

<style scoped>
.blame-view {
  height: 100%;
}

.blame-card {
  height: 100%;
  border-radius: var(--app-radius-lg);
}

:deep(.blame-card > .el-card__body) {
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

.empty-blame,
.loading-blame {
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

.blame-content {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  margin-top: var(--app-spacing-md);
}

.blame-header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  align-items: center;
  gap: var(--app-spacing);
  margin-bottom: var(--app-spacing);
}

.blame-tags {
  display: flex;
  flex-wrap: wrap;
  gap: var(--app-spacing-sm);
}

.path-tag {
  max-width: min(720px, 100%);
}

.blame-table {
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

:deep(.el-table__cell) {
  padding: 6px 0;
}

:deep(.el-table__fixed),
:deep(.el-table__fixed-right) {
  box-shadow: 6px 0 14px rgba(31, 35, 64, 0.06);
}

.line-number {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.author-cell {
  display: flex;
  align-items: center;
  gap: 6px;
}

.author-cell .el-icon {
  font-size: 14px;
  color: var(--el-text-color-secondary);
}

.code-line {
  display: block;
  min-width: max-content;
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
  white-space: pre;
  font-size: 13px;
  line-height: 1.5;
  color: var(--el-text-color-primary);
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
  
  .blame-header {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
