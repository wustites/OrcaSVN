<template>
  <div class="fork-shell">
    <header class="fork-toolbar">
      <div class="toolbar-group">
        <button class="brand-button" aria-label="OrcaSVN" @click="navigateTo('workspace')">
          <span class="brand-mark">
            <svg viewBox="0 0 512 512" aria-hidden="true">
              <path d="M91 297C132 217 214 170 345 166C322 187 309 211 306 237C347 241 385 258 420 289C358 288 315 305 288 340C221 361 155 347 91 297Z" />
              <path d="M153 288C188 238 233 209 290 201C270 232 269 263 286 294C235 310 191 308 153 288Z" class="brand-cut" />
              <path d="M326 207V143M326 143L374 114M326 143L281 112" class="brand-branch" />
              <circle cx="326" cy="207" r="18" class="brand-node" />
              <circle cx="374" cy="114" r="18" class="brand-node" />
              <circle cx="281" cy="112" r="18" class="brand-node" />
            </svg>
          </span>
          <span class="brand-name">OrcaSVN</span>
        </button>
        <span class="toolbar-divider"></span>
        <button class="tool-button" :class="{ active: routeName === 'workspace' }" @click="navigateTo('workspace')">
          <el-icon><FolderOpened /></el-icon>
          <span>{{ $t('menu.workspace') }}</span>
        </button>
        <button class="tool-button" @click="refreshStatus" :disabled="workspaceStore.isLoading">
          <el-icon><Refresh /></el-icon>
          <span>{{ $t('menu.refresh') }}</span>
        </button>
        <el-dropdown
          trigger="click"
          popper-class="toolbar-dropdown"
          :disabled="!workspaceStore.currentPath"
          @command="openCurrentWorkspaceIn"
        >
          <button class="tool-button" :disabled="!workspaceStore.currentPath">
            <el-icon><Operation /></el-icon>
            <span>{{ $t('menu.openIn') }}</span>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="explorer">
                {{ $t('openIn.explorer') }}
              </el-dropdown-item>
              <el-dropdown-item command="vscode">
                {{ $t('openIn.vscode') }}
              </el-dropdown-item>
              <el-dropdown-item command="terminal">
                {{ $t('openIn.terminal') }}
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <button class="tool-button" :class="{ active: routeName === 'log' }" @click="navigateTo('log')">
          <el-icon><Document /></el-icon>
          <span>{{ $t('menu.log') }}</span>
        </button>
      </div>

      <button class="repository-title" @click="navigateTo('workspace')">
        <strong>{{ repositoryName }}</strong>
        <span v-if="workspaceStore.svnInfo">r{{ workspaceStore.svnInfo.revision }}</span>
        <span v-else>OrcaSVN</span>
      </button>

      <div class="toolbar-group toolbar-group-right">
        <button class="tool-button" :class="{ active: routeName === 'commit' }" @click="navigateTo('commit')">
          <el-icon><Upload /></el-icon>
          <span>{{ $t('menu.commit') }}</span>
        </button>
        <button class="tool-button" :class="{ active: routeName === 'diff' }" @click="navigateTo('diff')">
          <el-icon><Connection /></el-icon>
          <span>{{ $t('menu.diff') }}</span>
        </button>
        <button class="tool-button" :class="{ active: routeName === 'blame' }" @click="navigateTo('blame')">
          <el-icon><Edit /></el-icon>
          <span>{{ $t('menu.blame') }}</span>
        </button>
        <span class="toolbar-divider"></span>
        <button class="tool-button" :class="{ active: routeName === 'settings' }" @click="navigateTo('settings')">
          <el-icon><Setting /></el-icon>
          <span>{{ $t('menu.settings') }}</span>
        </button>
        <LanguageSwitcher />
      </div>
    </header>

    <main class="fork-content">
      <aside class="shell-sidebar">
        <div class="shell-repository">
          <span class="repository-icon"><el-icon><Folder /></el-icon></span>
          <span class="repository-copy">
            <strong>{{ repositoryName }}</strong>
            <small v-if="workspaceStore.svnInfo">r{{ workspaceStore.svnInfo.revision }}</small>
            <small v-else>Subversion client</small>
          </span>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">WORKING COPY</div>
          <button :class="{ active: routeName === 'workspace' }" @click="navigateTo('workspace')">
            <el-icon><Document /></el-icon>
            <span>{{ $t('workspace.fileStatus') }}</span>
            <b>{{ workspaceStore.statusList.length }}</b>
          </button>
          <button :class="{ active: routeName === 'log' }" @click="navigateTo('log')">
            <el-icon><List /></el-icon>
            <span>{{ $t('menu.log') }}</span>
          </button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">REPOSITORY</div>
          <button :class="{ active: routeName === 'commit' }" @click="navigateTo('commit')">
            <el-icon><Upload /></el-icon>
            <span>{{ $t('menu.commit') }}</span>
          </button>
          <button :class="{ active: routeName === 'diff' }" @click="navigateTo('diff')">
            <el-icon><Connection /></el-icon>
            <span>{{ $t('menu.diff') }}</span>
          </button>
          <button :class="{ active: routeName === 'blame' }" @click="navigateTo('blame')">
            <el-icon><Edit /></el-icon>
            <span>{{ $t('menu.blame') }}</span>
          </button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">APPLICATION</div>
          <button :class="{ active: routeName === 'checkout' }" @click="navigateTo('checkout')">
            <el-icon><Download /></el-icon>
            <span>{{ $t('menu.checkout') }}</span>
          </button>
          <button :class="{ active: routeName === 'settings' }" @click="navigateTo('settings')">
            <el-icon><Setting /></el-icon>
            <span>{{ $t('menu.settings') }}</span>
          </button>
        </div>
      </aside>

      <section class="route-workbench">
        <header class="route-header">
          <div class="route-title">
            <span class="route-accent"></span>
            <strong>{{ currentRouteTitle }}</strong>
          </div>
          <span class="route-repository">{{ repositoryName }}</span>
        </header>
        <div class="route-content">
          <router-view v-slot="{ Component, route }">
            <keep-alive :include="cachedViews">
              <component :is="Component" :key="route.path" />
            </keep-alive>
          </router-view>
        </div>
      </section>
    </main>

    <footer class="fork-status">
      <span>{{ workspaceStore.currentPath || $t('workspace.noWorkspace') }}</span>
      <div>
        <span v-if="workspaceStore.hasChanges">{{ workspaceStore.statusList.length }} changes</span>
        <span v-if="workspaceStore.svnInfo">r{{ workspaceStore.svnInfo.revision }}</span>
        <span>v{{ appVersion }}</span>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import { openWorkspaceTarget, type OpenWorkspaceTarget } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useWorkspace } from '@/composables/useWorkspace'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'
import packageInfo from '../../package.json'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const workspaceStore = useWorkspaceStore()
const { refreshStatus, restoreLastWorkspace } = useWorkspace()
const appVersion = packageInfo.version
const cachedViews = ref(['WorkspaceView', 'LogView'])
const statusRefreshIntervalMs = 30_000
let statusRefreshTimer: number | undefined

const repositoryName = computed(() => {
  const path = workspaceStore.currentPath
  if (!path) return 'Welcome'
  return path.split(/[\\/]/).filter(Boolean).pop() || path
})

const routeName = computed(() => String(route.name || 'workspace'))
const currentRouteTitle = computed(() => {
  const titleKey = route.meta.title
  return t(typeof titleKey === 'string' ? titleKey : 'menu.workspace')
})

const navigateTo = (name: string) => router.push({ name })

const openCurrentWorkspaceIn = async (target: OpenWorkspaceTarget) => {
  if (!workspaceStore.currentPath) return

  try {
    await openWorkspaceTarget(workspaceStore.currentPath, target)
  } catch (err) {
    ElMessage.error(`${t('common.error')}：${err}`)
  }
}

const refreshStatusSilently = async () => {
  if (!workspaceStore.currentPath || workspaceStore.isLoading) return
  await refreshStatus()
}

onMounted(async () => {
  await restoreLastWorkspace()
  statusRefreshTimer = window.setInterval(refreshStatusSilently, statusRefreshIntervalMs)
})

onUnmounted(() => {
  if (statusRefreshTimer !== undefined) {
    window.clearInterval(statusRefreshTimer)
  }
})
</script>

<style scoped>
.fork-shell {
  display: grid;
  grid-template-rows: 56px minmax(0, 1fr) 24px;
  height: 100vh;
  overflow: hidden;
  color: #1f2937;
  background: #f8fafc;
}

.fork-toolbar {
  position: relative;
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  padding: 4px 8px;
  background: #f7f9fc;
  border-bottom: 1px solid #cbd5e1;
  box-shadow: 0 1px 2px rgba(15, 23, 42, .04);
}

.toolbar-group {
  align-items: stretch;
  display: flex;
  min-width: 330px;
  gap: 2px;
}

.toolbar-group-right {
  justify-content: flex-end;
}

.brand-button {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 104px;
  padding: 0 9px 0 5px;
  border: 0;
  border-radius: 5px;
  color: #0f2740;
  background: transparent;
  cursor: pointer;
}

.brand-button:hover {
  background: #edf3f9;
}

.brand-mark {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  color: #fff;
  background: linear-gradient(145deg, #126f9b, #074968);
  box-shadow: 0 1px 2px rgba(7, 73, 104, .28);
}

.brand-mark svg {
  width: 24px;
  fill: currentColor;
}

.brand-mark .brand-cut {
  fill: #0b5b80;
}

.brand-mark .brand-branch {
  fill: none;
  stroke: #73d6c9;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 20px;
}

.brand-mark .brand-node {
  fill: #fff;
  stroke: #73d6c9;
  stroke-width: 13px;
}

.brand-name {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: -.01em;
}

.toolbar-divider {
  width: 1px;
  height: 30px;
  align-self: center;
  margin: 0 5px;
  background: #d5dde6;
}

.tool-button {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-width: 54px;
  gap: 1px;
  border: 0;
  border-radius: 5px;
  color: #526173;
  background: transparent;
  font-size: 10px;
  cursor: pointer;
}

.tool-button:hover {
  background: #edf3f9;
  color: #123a55;
}

.tool-button:disabled {
  opacity: .45;
  cursor: not-allowed;
}

.tool-button:disabled:hover {
  color: #526173;
  background: transparent;
}

.tool-button.active {
  color: #075a82;
  background: #e2f0f7;
}

.tool-button .el-icon {
  font-size: 18px;
}

.repository-title {
  position: absolute;
  top: 6px;
  left: 50%;
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 220px;
  padding: 4px 30px;
  transform: translateX(-50%);
  border: 0;
  border: 1px solid #d6dee7;
  border-radius: 6px;
  background: #fff;
  color: #607083;
  font-size: 10px;
  cursor: pointer;
}

.repository-title strong {
  color: #21354a;
  font-size: 12px;
}

.fork-content {
  display: grid;
  grid-template-columns: 216px minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
  background: #f8fafc;
}

.shell-sidebar {
  min-height: 0;
  overflow: auto;
  background: #f1f5f9;
  border-right: 1px solid #cbd5e1;
}

.shell-repository {
  display: flex;
  align-items: center;
  min-height: 50px;
  gap: 9px;
  padding: 7px 10px;
  border-bottom: 1px solid #d8e0e8;
  color: #21354a;
  font-size: 11px;
}

.repository-icon {
  display: grid;
  place-items: center;
  flex: 0 0 28px;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  color: #fff;
  background: #0b668f;
}

.repository-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
}

.repository-copy strong,
.repository-copy small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.repository-copy small {
  color: #718096;
  font-size: 9px;
}

.sidebar-section {
  padding: 9px 7px 2px;
}

.sidebar-heading {
  padding: 3px 7px 5px;
  color: #7a899b;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: .06em;
}

.sidebar-section button {
  display: flex;
  align-items: center;
  width: 100%;
  height: 28px;
  gap: 7px;
  padding: 0 9px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: #425466;
  font-size: 11px;
  text-align: left;
}

.sidebar-section button:hover {
  background: #e6edf4;
}

.sidebar-section button.active {
  color: #fff;
  background: #0b668f;
  box-shadow: 0 1px 2px rgba(11, 102, 143, .2);
}

.sidebar-section button span {
  flex: 1;
}

.sidebar-section button b {
  font-size: 10px;
}

.route-workbench {
  display: grid;
  grid-template-rows: 38px minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
}

.route-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  background: #fff;
  border-bottom: 1px solid #d5dde6;
  color: #26394d;
  font-size: 11px;
}

.route-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.route-accent {
  width: 3px;
  height: 15px;
  border-radius: 2px;
  background: #0b668f;
}

.route-repository {
  color: #8290a1;
  font-size: 10px;
}

.route-content {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  background: #fff;
}

.route-content :deep(.left-panel),
.route-content :deep(.welcome-sidebar) {
  display: none;
}

.route-content :deep(.workspace-layout) {
  height: 100%;
}

.route-content :deep(.empty-state) {
  display: block;
  height: 100%;
}

.route-content :deep(.empty-content) {
  margin: 42px 44px;
}

.route-content :deep(.checkout-view),
.route-content :deep(.commit-view),
.route-content :deep(.settings-view),
.route-content :deep(.log-view),
.route-content :deep(.diff-view),
.route-content :deep(.blame-view) {
  width: 100%;
  max-width: none;
  min-height: 100%;
  margin: 0;
  padding: 0;
}

.route-content :deep(.el-card) {
  min-height: 100%;
  border: 0;
  border-radius: 0;
}

.route-content :deep(.el-card__header) {
  padding: 8px 12px;
}

.route-content :deep(.el-card__body) {
  padding: 12px;
}

.route-content :deep(.checkout-view),
.route-content :deep(.commit-view) {
  display: flex;
  min-height: 100%;
  background: var(--md-sys-color-surface);
}

.route-content :deep(.checkout-card),
.route-content :deep(.commit-card) {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 100%;
  background: var(--md-sys-color-surface);
}

.route-content :deep(.checkout-card > .el-card__body),
.route-content :deep(.commit-card > .el-card__body) {
  flex: 1;
  background: var(--md-sys-color-surface);
}

.fork-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 9px;
  border-top: 1px solid #cbd5e1;
  background: #eef3f7;
  color: #65768a;
  font-size: 10px;
}

.fork-status div {
  display: flex;
  gap: 14px;
}

@media (max-width: 900px) {
  .repository-title,
  .tool-button span,
  .brand-name {
    display: none;
  }
  .toolbar-group {
    min-width: 0;
  }
  .tool-button {
    min-width: 38px;
  }
  .fork-content {
    grid-template-columns: 1fr;
  }
  .shell-sidebar {
    display: none;
  }
}
</style>
