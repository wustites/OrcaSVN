<template>
  <div class="commit-view">
    <el-card class="commit-card">
      <template #header>
        <span>{{ $t('commit.title') }}</span>
      </template>

      <div v-if="!workspaceStore.currentPath" class="no-workspace">
        <el-empty :description="$t('log.openWorkspaceFirst')">
          <el-button type="primary" @click="openWorkspace">{{ $t('common.open') }}</el-button>
        </el-empty>
      </div>

      <div v-else>
        <el-alert
          :title="$t('commit.commitMessage')"
          type="info"
          :closable="false"
          class="commit-info"
        >
          {{ $t('commit.currentWorkspace') }}：<strong>{{ workspaceStore.currentPath }}</strong>
          <br>
          {{ $t('commit.changedFiles') }}：{{ changedFiles.length }} {{ $t('commit.filesCount', { count: changedFiles.length }) }}
        </el-alert>

        <el-form class="commit-form">
          <el-form-item :label="$t('commit.selectFiles')">
            <el-checkbox-group v-model="selectedFiles">
              <el-table :data="changedFiles" style="width: 100%">
                <el-table-column type="selection" width="55" />
                <el-table-column prop="status_code" :label="$t('commit.status')" width="60">
                  <template #default="{ row }">
                    <span :class="getStatusClass(row.status_code)">{{ row.status_code }}</span>
                  </template>
                </el-table-column>
                <el-table-column prop="path" :label="$t('commit.file')" />
              </el-table>
            </el-checkbox-group>
          </el-form-item>

          <el-form-item :label="$t('commit.commitMessage')" required>
            <el-input
              v-model="commitMessage"
              type="textarea"
              :rows="5"
              :placeholder="$t('commit.enterCommitMessage')"
            />
          </el-form-item>

          <el-form-item>
            <el-button type="primary" @click="doCommit" :loading="loading">
              <el-icon><Upload /></el-icon>
              {{ $t('common.commit') }}
            </el-button>
            <el-button @click="resetForm">{{ $t('common.reset') }}</el-button>
          </el-form-item>
        </el-form>

        <div v-if="output" class="output-area">
          <el-input
            v-model="output"
            type="textarea"
            :rows="8"
            readonly
            class="output-textarea"
          />
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnCommit } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'

const router = useRouter()
const workspaceStore = useWorkspaceStore()

const selectedFiles = ref<string[]>([])
const commitMessage = ref('')
const loading = ref(false)
const output = ref('')

const changedFiles = computed(() => {
  return workspaceStore.statusList.filter(
    s => s.status_code !== ' ' && s.status_code !== ''
  )
})

const openWorkspace = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择 SVN 工作区目录'
  })

  if (selected) {
    const path = Array.isArray(selected) ? selected[0] : selected
    workspaceStore.setCurrentPath(path)
    router.push({ name: 'workspace' })
  }
}

const doCommit = async () => {
  if (!workspaceStore.currentPath || !commitMessage.value) {
    return
  }

  loading.value = true
  output.value = ''

  try {
    const files = selectedFiles.value.length > 0 ? selectedFiles.value : undefined
    const result = await svnCommit(workspaceStore.currentPath, commitMessage.value, files)
    output.value = result.output

    // 刷新状态
    setTimeout(() => {
      router.push({ name: 'workspace' })
    }, 1000)
  } catch (err) {
    output.value = `错误：${err}`
  } finally {
    loading.value = false
  }
}

const resetForm = () => {
  selectedFiles.value = []
  commitMessage.value = ''
  output.value = ''
}

const getStatusClass = (statusCode: string): string => {
  switch (statusCode) {
    case 'A': return 'status-added'
    case 'M': return 'status-modified'
    case 'D': return 'status-deleted'
    default: return ''
  }
}

onMounted(() => {
  // 默认选中所有更改的文件
  selectedFiles.value = changedFiles.value.map(f => f.path)
})
</script>

<style scoped>
.commit-view {
  max-width: 900px;
  margin: 0 auto;
}

.no-workspace {
  padding: 40px 0;
}

.commit-info {
  margin-bottom: 20px;
}

.commit-form {
  margin-top: 20px;
}

.output-area {
  margin-top: 20px;
}

.output-textarea {
  font-family: 'Consolas', 'Monaco', monospace;
}
</style>
