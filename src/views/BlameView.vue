<template>
  <div class="blame-view">
    <el-card class="blame-card">
      <template #header>
        <div class="card-header">
          <span>{{ $t('blame.title') }}</span>
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
            <el-button @click="loadBlame" :loading="loading">
              <el-icon><Edit /></el-icon>
              {{ $t('common.load') }}
            </el-button>
          </div>
        </div>
      </template>

      <div v-if="!blameLines.length && !loading" class="empty-blame">
        <el-empty :description="$t('blame.selectFileToBlame')" />
      </div>

      <div v-else-if="loading" class="loading-blame">
        <el-skeleton :rows="20" animated />
      </div>

      <div v-else class="blame-content">
        <el-table :data="blameLines" style="width: 100%" max-height="700">
          <el-table-column prop="revision" :label="$t('blame.revision')" width="80" sortable />
          <el-table-column prop="author" :label="$t('blame.author')" width="120" />
          <el-table-column prop="line" :label="$t('blame.content')">
            <template #default="{ row }">
              <span class="code-line">{{ row.line }}</span>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { svnBlame } from '@/api/svn'
import type { BlameLine } from '@/types'

const route = useRoute()

const currentPath = ref('')
const loading = ref(false)
const blameLines = ref<BlameLine[]>([])

const loadBlame = async () => {
  const path = currentPath.value || route.query.path as string
  if (!path) return

  loading.value = true
  blameLines.value = []

  try {
    blameLines.value = await svnBlame(path)
  } catch (err) {
    console.error('加载注解失败:', err)
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

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.empty-blame,
.loading-blame {
  padding: 40px 0;
}

.blame-content {
  margin-top: 20px;
}

.code-line {
  font-family: 'Consolas', 'Monaco', monospace;
  white-space: pre;
}
</style>
