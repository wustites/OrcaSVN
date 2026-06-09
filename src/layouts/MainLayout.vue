<template>
  <div class="app-container">
    <!-- 顶部工具栏 -->
    <header class="toolbar">
      <div class="toolbar-left">
        <div class="logo">
          <el-icon class="logo-icon"><FolderOpened /></el-icon>
          <span class="logo-text">OrcaSVN</span>
        </div>
        <div class="toolbar-divider"></div>
        <div class="toolbar-actions">
          <el-tooltip :content="$t('menu.workspace')" placement="bottom">
            <el-button text @click="navigateTo('workspace')">
              <el-icon><HomeFilled /></el-icon>
            </el-button>
          </el-tooltip>
          <el-tooltip :content="$t('menu.checkout')" placement="bottom">
            <el-button text @click="navigateTo('checkout')">
              <el-icon><Download /></el-icon>
            </el-button>
          </el-tooltip>
          <el-tooltip :content="$t('menu.commit')" placement="bottom">
            <el-button text @click="navigateTo('commit')">
              <el-icon><Upload /></el-icon>
            </el-button>
          </el-tooltip>
          <el-tooltip :content="$t('menu.log')" placement="bottom">
            <el-button text @click="navigateTo('log')">
              <el-icon><Document /></el-icon>
            </el-button>
          </el-tooltip>
          <el-tooltip :content="$t('menu.diff')" placement="bottom">
            <el-button text @click="navigateTo('diff')">
              <el-icon><Connection /></el-icon>
            </el-button>
          </el-tooltip>
          <el-tooltip :content="$t('menu.blame')" placement="bottom">
            <el-button text @click="navigateTo('blame')">
              <el-icon><Edit /></el-icon>
            </el-button>
          </el-tooltip>
        </div>
      </div>
      <div class="toolbar-right">
        <div class="workspace-info" v-if="workspaceStore.currentPath">
          <el-icon><FolderOpened /></el-icon>
          <span class="workspace-path">{{ workspaceStore.currentPath }}</span>
        </div>
        <el-tooltip :content="$t('menu.refresh')" placement="bottom">
          <el-button text circle @click="refreshStatus" :loading="workspaceStore.isLoading">
            <el-icon><Refresh /></el-icon>
          </el-button>
        </el-tooltip>
        <el-tooltip :content="$t('menu.settings')" placement="bottom">
          <el-button text circle @click="openSettings">
            <el-icon><Setting /></el-icon>
          </el-button>
        </el-tooltip>
        <LanguageSwitcher />
      </div>
    </header>

    <!-- 主内容区域 -->
    <div class="main-content">
      <router-view v-slot="{ Component, route }">
        <transition name="fade" mode="out-in">
          <keep-alive :include="cachedViews">
            <component :is="Component" :key="route.path" />
          </keep-alive>
        </transition>
      </router-view>
    </div>

    <!-- 底部状态栏 -->
    <footer class="status-bar">
      <div class="status-left">
        <span class="status-item" v-if="workspaceStore.currentPath">
          <el-icon><FolderOpened /></el-icon>
          {{ workspaceStore.currentPath }}
        </span>
        <span class="status-item" v-else>
          <el-icon><InfoFilled /></el-icon>
          {{ $t('workspace.noWorkspace') }}
        </span>
      </div>
      <div class="status-right">
        <span class="status-item" v-if="workspaceStore.svnInfo">
          <el-icon><Connection /></el-icon>
          r{{ workspaceStore.svnInfo.revision }}
        </span>
        <span class="status-item" v-if="workspaceStore.modifiedCount > 0">
          <el-icon><Edit /></el-icon>
          {{ workspaceStore.modifiedCount }} {{ $t('workspace.statusModified') }}
        </span>
        <span class="status-item" v-if="workspaceStore.addedCount > 0">
          <el-icon><Plus /></el-icon>
          {{ workspaceStore.addedCount }} {{ $t('workspace.statusAdded') }}
        </span>
        <span class="status-item version">v0.2.4</span>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'
import { useWorkspace } from '@/composables/useWorkspace'
import {
  Connection,
  Document,
  Download,
  Edit,
  HomeFilled,
  Upload,
} from '@element-plus/icons-vue'

const router = useRouter()
const route = useRoute()
const workspaceStore = useWorkspaceStore()
const { refreshStatus } = useWorkspace()

const cachedViews = ref(['WorkspaceView', 'LogView'])

const navigateTo = (name: string) => {
  router.push({ name })
}

const openSettings = () => {
  router.push({ name: 'settings' })
}
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

/* 工具栏 */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 12px;
  background: var(--md-sys-color-surface);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  flex-shrink: 0;
}

.toolbar-left,
.toolbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-right: 8px;
}

.logo-icon {
  font-size: 20px;
  color: var(--md-sys-color-primary);
}

.logo-text {
  font-size: 14px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.toolbar-divider {
  width: 1px;
  height: 20px;
  background: var(--md-sys-color-outline-variant);
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}

.toolbar-actions .el-button {
  height: 32px;
  padding: 0 8px;
}

.workspace-info {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: var(--el-fill-color-light);
  border-radius: var(--app-radius-sm);
  font-size: 12px;
  color: var(--el-text-color-regular);
}

.workspace-info .el-icon {
  font-size: 14px;
  color: var(--md-sys-color-primary);
}

.workspace-path {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 主内容 */
.main-content {
  flex: 1;
  overflow: hidden;
}

/* 状态栏 */
.status-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 24px;
  padding: 0 12px;
  background: var(--md-sys-color-surface-container);
  border-top: 1px solid var(--md-sys-color-outline-variant);
  font-size: 11px;
  color: var(--el-text-color-secondary);
  flex-shrink: 0;
}

.status-left,
.status-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-item .el-icon {
  font-size: 12px;
}

.status-item.version {
  padding: 0 6px;
  background: var(--md-sys-color-primary-container);
  color: var(--md-sys-color-primary);
  border-radius: var(--app-radius-xs);
  font-weight: 600;
}

/* 过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* 响应式 */
@media (max-width: 860px) {
  .toolbar-actions {
    display: none;
  }
  
  .workspace-info {
    display: none;
  }
}
</style>
