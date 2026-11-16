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
        <button
          class="tool-button"
          :class="{ 'is-refreshing': workspaceStore.isLoading }"
          :aria-busy="workspaceStore.isLoading"
          :aria-label="$t('menu.refresh')"
          :title="$t('menu.refresh')"
          @click="refreshStatus"
          :disabled="workspaceStore.isLoading"
        >
          <el-icon><Refresh /></el-icon>
          <span>{{ $t('menu.refresh') }}</span>
        </button>
        <WorkspaceOpenMenu :path="workspaceStore.currentPath" />
      </div>

      <div class="toolbar-group toolbar-group-right">
        <button class="tool-button" :class="{ active: routeName === 'commit' }" @click="navigateTo('commit')">
          <el-icon><Upload /></el-icon>
          <span>{{ $t('menu.commit') }}</span>
        </button>
        <button class="tool-button" :class="{ active: routeName === 'update' }" @click="navigateTo('update')">
          <el-icon><RefreshRight /></el-icon>
          <span>{{ $t('menu.update') }}</span>
        </button>
        <span class="toolbar-divider"></span>
        <button class="tool-button" :class="{ active: routeName === 'settings' }" @click="navigateTo('settings')">
          <el-icon><Setting /></el-icon>
          <span>{{ $t('menu.settings') }}</span>
        </button>
        <LanguageSwitcher />
      </div>
    </header>

    <main class="fork-content" :style="{ '--workspace-rail-width': workspaceStore.workspaceListCollapsed ? '44px' : '280px' }">
      <WorkspaceSidebar class="workspace-rail" @open="handleWorkspaceCommand('__open__')" @select="handleWorkspaceCommand" />
      <aside class="shell-sidebar">
        <div class="shell-repository">
          <span class="repository-icon"><el-icon><Folder /></el-icon></span>
          <span class="repository-copy">
            <strong>{{ repositoryName }}</strong>
            <small v-if="workspaceStore.svnInfo">r{{ workspaceStore.svnInfo.revision }}</small>
            <small v-else>{{ $t('workspace.svnClient') }}</small>
          </span>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">{{ $t('workspace.workingCopy') }}</div>
          <button :class="{ active: routeName === 'workspace' }" @click="navigateTo('workspace')">
            <el-icon><FolderOpened /></el-icon>
            <span>{{ $t('menu.workspace') }}</span>
            <b>{{ workspaceStore.statusList.length }}</b>
          </button>
          <button :class="{ active: routeName === 'log' }" @click="navigateTo('log')">
            <el-icon><List /></el-icon>
            <span>{{ $t('menu.log') }}</span>
          </button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">{{ $t('workspace.repository') }}</div>
          <button :class="{ active: routeName === 'commit' }" @click="navigateTo('commit')">
            <el-icon><Upload /></el-icon>
            <span>{{ $t('menu.commit') }}</span>
          </button>
          <button :class="{ active: routeName === 'update' }" @click="navigateTo('update')">
            <el-icon><RefreshRight /></el-icon>
            <span>{{ $t('menu.update') }}</span>
          </button>
          <button :class="{ active: routeName === 'diff' }" @click="navigateTo('diff')">
            <el-icon><Connection /></el-icon>
            <span>{{ $t('menu.diff') }}</span>
          </button>
          <button :class="{ active: routeName === 'stash' }" @click="navigateTo('stash')">
            <el-icon><Archive /></el-icon>
            <span>{{ $t('menu.stash') }}</span>
            <b v-if="workspaceStore.currentPath">{{ stashCount }}</b>
          </button>
          <button :class="{ active: routeName === 'blame' }" @click="navigateTo('blame')">
            <el-icon><Edit /></el-icon>
            <span>{{ $t('menu.blame') }}</span>
          </button>
        </div>
        <div class="sidebar-section">
          <div class="sidebar-heading">{{ $t('workspace.application') }}</div>
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
        <span v-if="workspaceStore.hasChanges">{{ $t('workspace.changeCount', { count: workspaceStore.statusList.length }) }}</span>
        <span v-if="workspaceStore.svnInfo">r{{ workspaceStore.svnInfo.revision }}</span>
        <button
          v-if="updateState === 'available'"
          type="button"
          class="version-update"
          :title="$t('appUpdate.viewRelease')"
          @click="openReleasePage"
        >
          v{{ appVersion }}
          <span class="update-dot" aria-hidden="true"></span>
          {{ $t('appUpdate.available', { version: latestVersion }) }}
        </button>
        <span v-else>v{{ appVersion }}</span>
      </div>
    </footer>
    <OpenInSettingsDialog />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import { launchWorkspacePath } from '@/api/svn'
import { useWorkspaceStore } from '@/stores/workspace'
import { useStashStore } from '@/stores/stash'
import { useWorkspace } from '@/composables/useWorkspace'
import { useWorkspaceErrorToast } from '@/composables/useWorkspaceErrorToast'
import { useUpdateCheck } from '@/composables/useUpdateCheck'
import LanguageSwitcher from '@/components/LanguageSwitcher.vue'
import WorkspaceSidebar from '@/components/WorkspaceSidebar.vue'
import { Archive } from '@/components/icons/materialIcons'
import WorkspaceOpenMenu from '@/components/WorkspaceOpenMenu.vue'
import OpenInSettingsDialog from '@/components/OpenInSettingsDialog.vue'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const workspaceStore = useWorkspaceStore()
const stashStore = useStashStore()
const { loadWorkspace, openWorkspace, refreshStatus, refreshStatusIfStale, openStartupWorkspace } = useWorkspace()
const {
  currentVersion: appVersion,
  state: updateState,
  latestVersion,
  checkForUpdates,
  openReleasePage,
} = useUpdateCheck()
const cachedViews = ref(['WorkspaceView', 'LogView', 'UpdateView'])
const statusRefreshIntervalMs = 60_000
let statusRefreshTimer: number | undefined
useWorkspaceErrorToast()
const stashCount = computed(() => stashStore.currentEntries(workspaceStore.currentPath).length)

const repositoryName = computed(() => {
  const path = workspaceStore.currentPath
  if (!path) return t('workspace.welcomeRepository')
  return path.split(/[\\/]/).filter(Boolean).pop() || path
})

const routeName = computed(() => String(route.name || 'workspace'))
const currentRouteTitle = computed(() => {
  const titleKey = route.meta.title
  return t(typeof titleKey === 'string' ? titleKey : 'menu.workspace')
})

const navigateTo = (name: string) => router.push({ name })

const handleWorkspaceCommand = async (command: string) => {
  try {
    if (command === '__open__') {
      await openWorkspace(t('workspace.selectWorkspaceTitle'))
    } else if (command !== workspaceStore.currentPath) {
      await loadWorkspace(command)
    }
    await router.push({ name: 'workspace' })
  } catch (error) {
    ElMessage.error(String(error))
  }
}

const refreshStatusSilently = async () => {
  if (!workspaceStore.currentPath || workspaceStore.isLoading) return
  await refreshStatusIfStale(statusRefreshIntervalMs)
}

onMounted(async () => {
  void checkForUpdates()
  await openStartupWorkspace(await launchWorkspacePath().catch(() => null))
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
  grid-template-rows: var(--app-size-toolbar) minmax(0, 1fr) var(--app-size-statusbar);
  height: 100vh;
  overflow: hidden;
  color: var(--md-sys-color-on-surface);
  background: var(--md-sys-color-surface);
}

.fork-toolbar {
  position: relative;
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  padding: 0 18px;
  background: var(--md-sys-color-surface-container-lowest);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  box-shadow: none;
}

.toolbar-group {
  align-items: center;
  display: flex;
  min-width: 260px;
  gap: 6px;
}

.toolbar-group-right {
  justify-content: flex-end;
}

.brand-button {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 112px;
  padding: 0 10px 0 0;
  border: 0;
  border-radius: var(--app-radius-full);
  color: var(--md-sys-color-on-surface);
  background: transparent;
  cursor: pointer;
  transition:
    color var(--app-transition-fast),
    background-color var(--app-transition-fast);
}

.brand-button:hover {
  background: var(--md-sys-state-hover);
}

.brand-mark {
  display: grid;
  place-items: center;
  width: var(--app-size-control-compact);
  height: var(--app-size-control-compact);
  border-radius: 11px;
  color: var(--md-sys-color-on-primary);
  background: var(--md-sys-color-primary);
  box-shadow: none;
}

.brand-mark svg {
  width: var(--app-size-icon-lg);
  fill: currentColor;
}

.brand-mark .brand-cut {
  fill: var(--md-sys-color-primary-active);
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
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -.01em;
}

:global(.theme-dark .fork-toolbar .brand-button),
:global(.dark .fork-toolbar .brand-button),
:global(.theme-dark .fork-toolbar .brand-button .brand-name),
:global(.dark .fork-toolbar .brand-button .brand-name) {
  color: var(--md-sys-color-on-surface) !important;
}

:global(.theme-dark .fork-toolbar .brand-button:hover),
:global(.dark .fork-toolbar .brand-button:hover),
:global(.theme-dark .fork-toolbar .brand-button:focus-visible),
:global(.dark .fork-toolbar .brand-button:focus-visible) {
  color: var(--md-sys-color-on-surface) !important;
  background: var(--md-sys-state-hover) !important;
}

.toolbar-divider {
  width: 1px;
  height: 32px;
  align-self: center;
  margin: 0 5px;
  background: var(--md-sys-color-outline-variant);
}

.tool-button {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: center;
  min-width: var(--app-size-control);
  height: var(--app-size-control);
  gap: 8px;
  padding: 0 10px;
  border: 0;
  border-radius: var(--app-radius-md);
  color: var(--md-sys-color-on-surface-variant);
  background: transparent;
  font-size: var(--app-font-size-label);
  cursor: pointer;
  transition:
    color var(--app-transition-fast),
    background-color var(--app-transition-fast);
}

.tool-button:hover {
  background: var(--md-sys-state-hover);
  color: var(--md-sys-color-on-surface);
}

.tool-button:disabled {
  opacity: .45;
  cursor: not-allowed;
}

.tool-button:disabled:hover {
  color: var(--md-sys-color-on-surface-variant);
  background: transparent;
}

:global(.theme-dark) .tool-button:disabled,
:global(.dark) .tool-button:disabled {
  opacity: .5;
  color: var(--md-sys-color-on-surface-variant);
}

:global(.theme-dark) .tool-button:disabled:hover,
:global(.dark) .tool-button:disabled:hover {
  color: var(--md-sys-color-on-surface-variant);
  background: transparent;
}

.tool-button.is-refreshing:disabled {
  opacity: .82;
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
}

:global(.theme-dark) .tool-button.is-refreshing:disabled,
:global(.dark) .tool-button.is-refreshing:disabled {
  opacity: 1;
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
}

.tool-button.is-refreshing .el-icon {
  animation: toolbar-refresh-spin .8s linear infinite;
}

.tool-button.active {
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
}

.tool-button .el-icon {
  font-size: var(--app-font-size-title);
}

.open-in-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  width: auto;
  height: var(--app-size-control);
  flex-direction: row;
  gap: 6px;
  padding: 0 10px;
  border: 0;
  border-radius: var(--app-radius-md);
  color: var(--md-sys-color-on-surface-variant);
  background: transparent;
  cursor: pointer;
  transition:
    color var(--app-transition-fast),
    background-color var(--app-transition-fast),
    border-color var(--app-transition-fast);
}

.open-in-trigger:hover,
.open-in-trigger:focus-visible {
  color: var(--md-sys-color-on-surface);
  background: var(--md-sys-state-hover);
}

.open-in-trigger[aria-expanded="true"] {
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
}

.open-in-trigger:disabled {
  opacity: .45;
  cursor: not-allowed;
}

.open-in-trigger:disabled:hover {
  color: var(--md-sys-color-on-surface-variant);
  background: transparent;
}

.open-in-app-icon {
  font-size: 20px;
}

.open-in-icons {
  display: flex;
  align-items: center;
}

.open-in-label {
  max-width: 90px;
  overflow: hidden;
  font-size: var(--app-font-size-label);
  line-height: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.open-in-chevron {
  margin-left: -1px;
  font-size: 15px;
}

:global(.toolbar-dropdown .el-dropdown-menu__item .el-icon) {
  margin-right: 10px;
  color: inherit;
  font-size: 19px;
}

:global(.toolbar-dropdown .el-dropdown-menu__item.is-current-target) {
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
  font-weight: 600;
}

:global(.toolbar-dropdown .el-dropdown-menu__item.is-current-target .app-material-icon) {
  font-variation-settings:
    "FILL" 1,
    "wght" 500,
    "GRAD" 0,
    "opsz" 24;
}

@keyframes toolbar-refresh-spin {
  to {
    transform: rotate(360deg);
  }
}

.fork-content {
  display: grid;
  grid-template-columns: var(--workspace-rail-width, 280px) var(--app-size-sidebar) minmax(0, 1fr);
  grid-template-areas: "workspaces navigation content";
  min-height: 0;
  overflow: hidden;
  background: var(--md-sys-color-surface);
}

.workspace-rail {
  grid-area: workspaces;
}

.shell-sidebar {
  grid-area: navigation;
  min-height: 0;
  overflow: auto;
  padding: 14px 10px;
  background: var(--md-sys-color-surface-container-lowest);
  border-right: 1px solid var(--md-sys-color-outline-variant);
}

.shell-repository {
  display: flex;
  align-items: center;
  min-height: 72px;
  gap: 12px;
  padding: 8px 10px 12px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  color: var(--md-sys-color-on-surface);
  font-size: var(--app-font-size-label);
}

.repository-icon {
  display: grid;
  place-items: center;
  flex: 0 0 36px;
  width: 36px;
  height: 36px;
  border-radius: var(--app-radius-md);
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
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
  color: var(--md-sys-color-on-surface-variant);
  font-size: var(--app-font-size-xs);
}

.sidebar-section {
  padding: 18px 2px 2px;
}

.sidebar-heading {
  padding: 3px 12px 8px;
  color: var(--md-sys-color-on-surface-variant);
  font-size: var(--app-font-size-2xs);
  font-weight: 700;
  letter-spacing: .06em;
}

.sidebar-section button {
  display: flex;
  align-items: center;
  width: 100%;
  height: var(--app-size-field);
  gap: 10px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--app-radius-sm);
  background: transparent;
  color: var(--md-sys-color-on-surface-variant);
  font-size: var(--app-font-size-label);
  text-align: left;
}

.sidebar-section button:hover {
  color: var(--md-sys-color-on-surface);
  background: var(--md-sys-state-hover);
}

.sidebar-section button.active {
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
  box-shadow: none;
}

.sidebar-section button span {
  flex: 1;
}

.sidebar-section button b {
  font-size: var(--app-font-size-xs);
}

.route-workbench {
  grid-area: content;
  display: grid;
  grid-template-rows: var(--app-size-page-header) minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  margin: 6px;
  overflow: hidden;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-sm);
  background: var(--md-sys-color-surface-container-lowest);
  box-shadow: var(--md-sys-elevation-2);
}

.route-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  background: var(--md-sys-color-surface-container-lowest);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  color: var(--md-sys-color-on-surface);
  font-size: var(--app-font-size-title);
}

.route-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.route-accent {
  display: none;
}

.route-repository {
  color: var(--md-sys-color-on-surface-variant);
  font-size: var(--app-font-size-sm);
}

.route-content {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  background: var(--md-sys-color-surface-container-lowest);
}

.route-content :deep(.left-panel),
.route-content :deep(.welcome-sidebar) {
  display: none;
}

.route-content :deep(.workspace-layout) {
  height: 100%;
}

.route-content :deep(.empty-state) {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 0;
  padding: var(--app-spacing-2xl);
}

.route-content :deep(.empty-content) {
  margin: 0;
  width: min(100%, 570px);
}

.route-content :deep(.checkout-view),
.route-content :deep(.commit-view),
.route-content :deep(.update-view),
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
  padding: 14px 24px;
  background: var(--md-sys-color-surface-container-lowest);
}

.route-content :deep(.el-card__body) {
  padding: 24px;
}

.route-content :deep(.checkout-view),
.route-content :deep(.commit-view),
.route-content :deep(.update-view),
.route-content :deep(.settings-view) {
  display: flex;
  min-height: 100%;
  background: transparent;
}

.route-content :deep(.checkout-card),
.route-content :deep(.commit-card),
.route-content :deep(.update-card),
.route-content :deep(.settings-card) {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 100%;
  background: var(--md-sys-color-surface-container-lowest);
}

.route-content :deep(.checkout-card > .el-card__body),
.route-content :deep(.commit-card > .el-card__body),
.route-content :deep(.update-card > .el-card__body),
.route-content :deep(.settings-card > .el-card__body) {
  flex: 1;
  background: var(--md-sys-color-surface-container-lowest);
}

.fork-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-top: 1px solid var(--md-sys-color-outline-variant);
  background: var(--md-sys-color-surface-container);
  color: var(--md-sys-color-on-surface-variant);
  font-size: var(--app-font-size-xs);
}

.fork-status div {
  display: flex;
  gap: 14px;
  flex-shrink: 0;
}

.version-update {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0;
  border: 0;
  background: none;
  color: var(--md-sys-color-primary);
  font: inherit;
  cursor: pointer;
}

.version-update:hover {
  text-decoration: underline;
}

.update-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--md-sys-color-primary);
}

.fork-status > span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 900px) {
  .tool-button > span:not(.el-icon),
  .open-in-label,
  .brand-name {
    display: none;
  }
  .brand-button {
    min-width: 0;
    padding-right: 0;
  }
  .toolbar-group {
    min-width: 0;
  }
  .tool-button {
    min-width: 38px;
  }
  .fork-content {
    grid-template-columns: var(--workspace-rail-width, 280px) minmax(0, 1fr);
    grid-template-rows: 44px minmax(0, 1fr);
    grid-template-areas: "workspaces navigation" "workspaces content";
  }
  .shell-sidebar {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 4px 8px;
    border-right: 0;
    border-bottom: 1px solid var(--md-sys-color-outline-variant);
  }
  .shell-sidebar::-webkit-scrollbar {
    height: 4px;
  }
  .shell-repository,
  .shell-sidebar .sidebar-heading {
    display: none;
  }
  .shell-sidebar .sidebar-section {
    display: flex;
    flex: 0 0 auto;
    gap: 2px;
    padding: 0 5px 0 0;
    border-right: 1px solid var(--md-sys-color-outline-variant);
  }
  .shell-sidebar .sidebar-section:last-child {
    border-right: 0;
  }
  .shell-sidebar .sidebar-section button {
    width: auto;
    height: 34px;
    padding: 0 10px;
    white-space: nowrap;
  }
}

@media (max-width: 650px) {
  .fork-toolbar {
    align-items: flex-start;
    padding-right: 12px;
    padding-left: 12px;
  }
  .toolbar-group-right .tool-button,
  .toolbar-group-right .toolbar-divider {
    display: none;
  }
  .route-repository {
    max-width: 42%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .route-content :deep(.empty-state) {
    padding: var(--app-spacing-xl) var(--app-spacing-lg);
  }
  .route-content :deep(.empty-actions) {
    flex-wrap: wrap;
  }
}
</style>
