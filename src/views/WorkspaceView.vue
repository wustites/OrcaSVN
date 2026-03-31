<template>
  <div class="workspace-view">
    <div v-if="!workspaceStore.currentPath" class="empty-workspace">
      <el-empty :description="$t('workspace.emptyWorkspace')">
        <el-button type="primary" @click="openWorkspace">{{ $t('workspace.openWorkspace') }}</el-button>
        <el-button @click="doCheckout">{{ $t('common.checkout') }}</el-button>
      </el-empty>
    </div>

    <div v-else class="workspace-content">
      <el-card class="info-card">
        <template #header>
          <div class="card-header">
            <span>{{ $t('workspace.repositoryInfo') }}</span>
            <el-button @click="closeWorkspace">{{ $t('common.close') }}</el-button>
          </div>
        </template>
        <el-alert v-if="workspaceStore.error" type="error" :title="workspaceStore.error" :closable="false" class="mb-4" />
        <el-descriptions :column="2" border v-if="workspaceStore.svnInfo">
          <el-descriptions-item :label="$t('workspace.path')">{{ workspaceStore.svnInfo.path }}</el-descriptions-item>
          <el-descriptions-item label="URL">{{ workspaceStore.svnInfo.url }}</el-descriptions-item>
          <el-descriptions-item :label="$t('workspace.repositoryRoot')">{{ workspaceStore.svnInfo.repository_root }}</el-descriptions-item>
          <el-descriptions-item :label="$t('workspace.revision')">{{ workspaceStore.svnInfo.revision }}</el-descriptions-item>
          <el-descriptions-item :label="$t('workspace.type')">{{ workspaceStore.svnInfo.node_kind }}</el-descriptions-item>
        </el-descriptions>
        <div v-else-if="!workspaceStore.isLoading" class="no-info">{{ $t('workspace.noRepositoryInfo') }}</div>
      </el-card>

      <el-card class="status-card">
        <template #header>
          <div class="card-header">
            <span>{{ $t('workspace.fileStatus') }}</span>
            <div class="header-actions">
              <el-button @click="refreshStatus" :loading="workspaceStore.isLoading">
                <el-icon><Refresh /></el-icon>
                {{ $t('common.refresh') }}
              </el-button>
              <el-button type="primary" @click="doUpdate">
                <el-icon><RefreshRight /></el-icon>
                {{ $t('common.update') }}
              </el-button>
            </div>
          </div>
        </template>

        <div class="status-summary">
          <el-tag type="success" effect="plain">{{ $t('workspace.statusModified') }}：{{ workspaceStore.modifiedCount }}</el-tag>
          <el-tag type="warning" effect="plain">{{ $t('workspace.statusAdded') }}：{{ workspaceStore.addedCount }}</el-tag>
          <el-tag type="danger" effect="plain">{{ $t('workspace.statusDeleted') }}：{{ workspaceStore.deletedCount }}</el-tag>
        </div>

        <el-table :data="workspaceStore.statusList" style="width: 100%" max-height="400">
          <el-table-column prop="status_code" :label="$t('commit.status')" width="60">
            <template #default="{ row }">
              <span :class="getStatusClass(row.status_code)">{{ row.status_code }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="path" :label="$t('commit.file')" />
          <el-table-column :label="$t('common.action')" width="200">
            <template #default="{ row }">
              <el-button link @click="viewDiff(row.path)">{{ $t('common.diff') }}</el-button>
              <el-button link @click="viewBlame(row.path)">{{ $t('common.blame') }}</el-button>
              <el-button link type="danger" @click="revertFile(row.path)">{{ $t('common.revert') }}</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-card>

      <el-card class="actions-card">
        <template #header>
          <span>{{ $t('workspace.quickActions') }}</span>
        </template>
        <div class="quick-actions">
          <el-button @click="doCommit">
            <el-icon><Upload /></el-icon>
            {{ $t('common.commit') }}
          </el-button>
          <el-button @click="doAdd">
            <el-icon><Plus /></el-icon>
            {{ $t('common.add') }}
          </el-button>
          <el-button @click="doDelete">
            <el-icon><Delete /></el-icon>
            {{ $t('common.delete') }}
          </el-button>
          <el-button @click="doCleanup">
            <el-icon><Delete /></el-icon>
            {{ $t('common.cleanup') }}
          </el-button>
          <el-button @click="doSwitch">
            <el-icon><Switch /></el-icon>
            {{ $t('common.switch') }}
          </el-button>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo, svnUpdate, svnCleanup, svnRevert } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'

const router = useRouter()
const workspaceStore = useWorkspaceStore()

const openWorkspace = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择 SVN 工作区目录'
  })

  if (selected) {
    const path = Array.isArray(selected) ? selected[0] : selected
    console.log('选择的路径:', path)
    workspaceStore.setCurrentPath(path)

    try {
      console.log('开始加载 status...')
      const status = await svnStatus(path)
      console.log('status 加载完成:', status.length, 'items')
      workspaceStore.statusList = status

      console.log('开始加载 info...')
      const info = await svnInfo(path)
      console.log('info 加载完成:', info)
      workspaceStore.svnInfo = info

      console.log('加载完成，store 状态:', {
        currentPath: workspaceStore.currentPath,
        svnInfo: workspaceStore.svnInfo,
        statusList: workspaceStore.statusList.length
      })
    } catch (err) {
      console.error('加载工作区失败:', err)
      workspaceStore.error = String(err)
    }
  }
}

const doCheckout = () => {
  router.push({ name: 'checkout' })
}

const closeWorkspace = () => {
  workspaceStore.clearWorkspace()
}

const refreshStatus = async () => {
  if (!workspaceStore.currentPath) return

  workspaceStore.isLoading = true
  try {
    const status = await svnStatus(workspaceStore.currentPath)
    console.log('refresh status:', status.length, 'items')
    workspaceStore.statusList = status

    const info = await svnInfo(workspaceStore.currentPath)
    console.log('refresh info:', info)
    workspaceStore.svnInfo = info
  } catch (err) {
    console.error('刷新状态失败:', err)
  } finally {
    workspaceStore.isLoading = false
  }
}

const doUpdate = async () => {
  if (!workspaceStore.currentPath) return

  try {
    await svnUpdate(workspaceStore.currentPath)
    await refreshStatus()
  } catch (err) {
    console.error('更新失败:', err)
  }
}

const doCommit = () => {
  router.push({ name: 'commit' })
}

const doAdd = async () => {
  console.log('添加文件')
}

const doDelete = async () => {
  console.log('删除文件')
}

const doCleanup = async () => {
  if (!workspaceStore.currentPath) return

  try {
    await svnCleanup(workspaceStore.currentPath)
  } catch (err) {
    console.error('清理失败:', err)
  }
}

const doSwitch = async () => {
  console.log('切换分支')
}

const viewDiff = (path: string) => {
  router.push({ name: 'diff', query: { path } })
}

const viewBlame = (path: string) => {
  router.push({ name: 'blame', query: { path } })
}

const revertFile = async (path: string) => {
  if (!workspaceStore.currentPath) return

  try {
    await svnRevert(workspaceStore.currentPath, [path])
    await refreshStatus()
  } catch (err) {
    console.error('还原失败:', err)
  }
}

const getStatusClass = (statusCode: string): string => {
  switch (statusCode) {
    case 'A': return 'status-added'
    case 'M': return 'status-modified'
    case 'D': return 'status-deleted'
    case '?': return 'status-unversioned'
    default: return ''
  }
}
</script>

<style scoped>
.workspace-view {
  height: 100%;
}

.empty-workspace {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
}

.workspace-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  height: 100%;
  overflow-y: auto;
}

.info-card,
.status-card,
.actions-card {
  flex-shrink: 0;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.status-summary {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}

.quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.status-added {
  color: #67c23a;
  font-weight: bold;
}

.status-modified {
  color: #e6a23c;
  font-weight: bold;
}

.status-deleted {
  color: #f56c6c;
  font-weight: bold;
}

.status-unversioned {
  color: #909399;
}

.mb-4 {
  margin-bottom: 16px;
}

.no-info {
  color: #999;
  text-align: center;
  padding: 20px;
}
</style>
