<template>
  <el-container class="main-container">
    <el-aside width="200px" class="sidebar">
      <div class="logo">
        <el-icon><FolderOpened /></el-icon>
        <span>OrcaSVN</span>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="sidebar-menu"
        @select="handleMenuSelect"
      >
        <el-menu-item index="workspace">
          <el-icon><HomeFilled /></el-icon>
          <span>{{ $t('menu.workspace') }}</span>
        </el-menu-item>
        <el-menu-item index="checkout">
          <el-icon><Download /></el-icon>
          <span>{{ $t('menu.checkout') }}</span>
        </el-menu-item>
        <el-menu-item index="commit">
          <el-icon><Upload /></el-icon>
          <span>{{ $t('menu.commit') }}</span>
        </el-menu-item>
        <el-menu-item index="log">
          <el-icon><Document /></el-icon>
          <span>{{ $t('menu.log') }}</span>
        </el-menu-item>
        <el-menu-item index="diff">
          <el-icon><Connection /></el-icon>
          <span>{{ $t('menu.diff') }}</span>
        </el-menu-item>
        <el-menu-item index="blame">
          <el-icon><Edit /></el-icon>
          <span>{{ $t('menu.blame') }}</span>
        </el-menu-item>
      </el-menu>
    </el-aside>
    <el-container>
      <el-header class="header">
        <div class="header-left">
          <el-breadcrumb v-if="workspaceStore.currentPath">
            <el-breadcrumb-item>{{ workspaceStore.currentPath }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-button @click="refreshStatus" :loading="workspaceStore.isLoading">
            <el-icon><Refresh /></el-icon>
            {{ $t('menu.refresh') }}
          </el-button>
          <el-button @click="openSettings">
            <el-icon><Setting /></el-icon>
            {{ $t('menu.settings') }}
          </el-button>
          <LanguageSwitcher />
        </div>
      </el-header>
      <el-main class="main-content">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnStatus, svnInfo } from '@/api/svn'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'

const router = useRouter()
const route = useRoute()
const workspaceStore = useWorkspaceStore()

const activeMenu = computed(() => route.name as string)

const handleMenuSelect = (index: string) => {
  router.push({ name: index })
}

const refreshStatus = async () => {
  if (!workspaceStore.currentPath) return

  workspaceStore.isLoading = true
  try {
    const [status, info] = await Promise.all([
      svnStatus(workspaceStore.currentPath),
      svnInfo(workspaceStore.currentPath)
    ])
    workspaceStore.statusList = status
    workspaceStore.svnInfo = info
  } catch (err) {
    console.error('刷新状态失败:', err)
  } finally {
    workspaceStore.isLoading = false
  }
}

const openSettings = () => {
  router.push({ name: 'settings' })
}
</script>

<style scoped>
.main-container {
  height: 100vh;
}

.sidebar {
  background: #304156;
  color: #fff;
}

.logo {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 60px;
  font-size: 20px;
  font-weight: bold;
  gap: 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.sidebar-menu {
  border-right: none;
  background: transparent;
}

:deep(.el-menu-item) {
  color: rgba(255, 255, 255, 0.7);
}

:deep(.el-menu-item:hover),
:deep(.el-menu-item.is-active) {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #fff;
  border-bottom: 1px solid #e6e6e6;
  padding: 0 20px;
}

.header-left {
  flex: 1;
}

.header-right {
  display: flex;
  gap: 10px;
}

.main-content {
  background: #f5f7fa;
  padding: 20px;
  overflow: auto;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
