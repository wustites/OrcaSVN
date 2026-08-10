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
        <button class="tool-button" :class="{ active: routeName === 'log' }" @click="navigateTo('log')">
          <el-icon><Document /></el-icon>
          <span>{{ $t('menu.log') }}</span>
        </button>
        <button
          class="tool-button"
          :class="{ 'is-refreshing': workspaceStore.isLoading }"
          :aria-busy="workspaceStore.isLoading"
          @click="refreshStatus"
          :disabled="workspaceStore.isLoading"
        >
          <el-icon><Refresh /></el-icon>
          <span>{{ $t('menu.refresh') }}</span>
        </button>
        <el-dropdown
          trigger="click"
          popper-class="toolbar-dropdown"
          :disabled="!workspaceStore.currentPath"
          @command="openCurrentWorkspaceIn"
        >
          <button
            class="open-in-trigger"
            :disabled="!workspaceStore.currentPath"
            :aria-label="$t('menu.openIn')"
            :title="$t('menu.openIn')"
          >
            <span class="open-in-icons">
              <el-icon class="open-in-app-icon"><component :is="selectedOpenTargetIcon" /></el-icon>
              <el-icon class="open-in-chevron"><ArrowDown /></el-icon>
            </span>
            <span class="open-in-label">{{ $t('menu.openIn') }}</span>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="explorer" :class="{ 'is-current-target': selectedOpenTarget === 'explorer' }">
                <el-icon><component :is="openTargetIcons.explorer" /></el-icon>
                {{ $t('openIn.explorer') }}
              </el-dropdown-item>
              <el-dropdown-item command="vscode" :class="{ 'is-current-target': selectedOpenTarget === 'vscode' }">
                <el-icon><component :is="openTargetIcons.vscode" /></el-icon>
                {{ $t('openIn.vscode') }}
              </el-dropdown-item>
              <el-dropdown-item command="terminal" :class="{ 'is-current-target': selectedOpenTarget === 'terminal' }">
                <el-icon><component :is="openTargetIcons.terminal" /></el-icon>
                {{ $t('openIn.terminal') }}
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>

      <el-dropdown trigger="click" popper-class="workspace-switcher-dropdown" @command="handleWorkspaceCommand">
        <button class="repository-title" :title="$t('workspace.switchWorkspace')">
          <strong>{{ repositoryName }}</strong>
          <span>
            <template v-if="workspaceStore.svnInfo">r{{ workspaceStore.svnInfo.revision }}</template>
            <template v-else>OrcaSVN</template>
            <el-icon class="repository-chevron"><ArrowDown /></el-icon>
          </span>
        </button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
              v-for="path in workspaceStore.recentWorkspaces"
              :key="path"
              :command="path"
              :class="{ 'is-current-workspace': path === workspaceStore.currentPath }"
            >
              <span class="workspace-option">
                <span class="workspace-option-copy">
                  <strong>{{ workspaceNameFromPath(path) }}</strong>
                  <small>{{ path }}</small>
                </span>
                <button
                  type="button"
                  class="workspace-remove"
                  :title="$t('workspace.removeWorkspace')"
                  :aria-label="$t('workspace.removeWorkspace')"
                  @click.stop="removeWorkspace(path)"
                ><el-icon><Close /></el-icon></button>
              </span>
            </el-dropdown-item>
            <el-dropdown-item divided command="__open__">
              <el-icon><FolderOpened /></el-icon>
              {{ $t('workspace.addWorkspace') }}
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <div class="toolbar-group toolbar-group-right">
        <button class="tool-button" :class="{ active: routeName === 'commit' }" @click="navigateTo('commit')">
          <el-icon><Upload /></el-icon>
          <span>{{ $t('menu.commit') }}</span>
        </button>
        <button class="tool-button" :class="{ active: routeName === 'update' }" @click="navigateTo('update')">
          <el-icon><RefreshRight /></el-icon>
          <span>{{ $t('menu.update') }}</span>
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
import {
  Code,
  FolderOpened as FolderOpenedIcon,
  Terminal,
} from '@/components/icons/materialIcons'
import packageInfo from '../../package.json'

const router = useRouter()
const route = useRoute()
const { t } = useI18n()
const workspaceStore = useWorkspaceStore()
const { loadWorkspace, openWorkspace, refreshStatus, restoreLastWorkspace } = useWorkspace()
const appVersion = packageInfo.version
const cachedViews = ref(['WorkspaceView', 'LogView', 'UpdateView'])
const statusRefreshIntervalMs = 60_000
let statusRefreshTimer: number | undefined
const selectedOpenTarget = ref<OpenWorkspaceTarget>('explorer')
const openTargetIcons = {
  explorer: FolderOpenedIcon,
  vscode: Code,
  terminal: Terminal,
} satisfies Record<OpenWorkspaceTarget, typeof FolderOpenedIcon>
const selectedOpenTargetIcon = computed(() => openTargetIcons[selectedOpenTarget.value])

const repositoryName = computed(() => {
  const path = workspaceStore.currentPath
  if (!path) return t('workspace.welcomeRepository')
  return path.split(/[\\/]/).filter(Boolean).pop() || path
})

const workspaceNameFromPath = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() || path

const routeName = computed(() => String(route.name || 'workspace'))
const currentRouteTitle = computed(() => {
  const titleKey = route.meta.title
  return t(typeof titleKey === 'string' ? titleKey : 'menu.workspace')
})

const navigateTo = (name: string) => router.push({ name })

const handleWorkspaceCommand = async (command: string) => {
  if (command === '__open__') {
    await openWorkspace(t('workspace.selectWorkspaceTitle'))
  } else if (command !== workspaceStore.currentPath) {
    await loadWorkspace(command)
  }
  await router.push({ name: 'workspace' })
}

const removeWorkspace = (path: string) => workspaceStore.removeRecentWorkspace(path)

const openCurrentWorkspaceIn = async (target: OpenWorkspaceTarget) => {
  if (!workspaceStore.currentPath) return

  try {
    await openWorkspaceTarget(workspaceStore.currentPath, target)
    selectedOpenTarget.value = target
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
  grid-template-rows: 64px minmax(0, 1fr) 28px;
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
  padding: 6px 12px;
  background: var(--md-sys-color-surface-container-low);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  box-shadow: none;
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
  min-width: 112px;
  padding: 0 12px 0 4px;
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
  width: 38px;
  height: 38px;
  border-radius: var(--app-radius-md);
  color: var(--md-sys-color-on-primary);
  background: var(--md-sys-color-primary);
  box-shadow: var(--md-sys-elevation-1);
}

.brand-mark svg {
  width: 24px;
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
  font-size: 14px;
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
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-width: 58px;
  gap: 2px;
  border: 0;
  border-radius: var(--app-radius-md);
  color: var(--md-sys-color-on-surface-variant);
  background: transparent;
  font-size: 12px;
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
  font-size: 20px;
}

.open-in-trigger {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  align-self: center;
  width: 58px;
  height: 52px;
  flex-direction: column;
  gap: 1px;
  padding: 0;
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
  max-width: 56px;
  overflow: hidden;
  font-size: 11px;
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

.repository-title {
  position: absolute;
  top: 8px;
  left: 50%;
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 230px;
  min-height: 48px;
  padding: 5px 32px;
  transform: translateX(-50%);
  border: 0;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-lg);
  background: var(--md-sys-color-surface-container-lowest);
  color: var(--md-sys-color-on-surface-variant);
  font-size: 11px;
  cursor: pointer;
  transition:
    border-color var(--app-transition-fast),
    background-color var(--app-transition-fast),
    color var(--app-transition-fast);
}

.repository-title:hover {
  border-color: var(--md-sys-color-outline);
  background: var(--md-sys-color-surface-container-high);
}

.repository-title strong {
  color: var(--md-sys-color-on-surface);
  font-size: 13px;
}

.repository-chevron {
  margin-left: 4px;
  font-size: 12px;
  vertical-align: -2px;
}

:global(.workspace-switcher-dropdown .el-dropdown-menu) {
  min-width: 330px;
  max-width: min(520px, 90vw);
}

:global(.workspace-switcher-dropdown .el-dropdown-menu__item) {
  padding: 7px 12px;
}

:global(.workspace-switcher-dropdown .el-dropdown-menu__item.is-current-workspace) {
  color: var(--md-sys-color-on-primary-container);
  background: var(--md-sys-color-primary-container);
}

.workspace-option {
  display: flex;
  align-items: center;
  width: 100%;
  min-width: 0;
  gap: 12px;
}

.workspace-option-copy {
  display: flex;
  flex: 1;
  min-width: 0;
  flex-direction: column;
  line-height: 1.35;
}

.workspace-option-copy small {
  overflow: hidden;
  color: var(--md-sys-color-on-surface-variant);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workspace-remove {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 0;
  border-radius: var(--app-radius-full);
  color: inherit;
  background: transparent;
  cursor: pointer;
}

.workspace-remove:hover {
  background: var(--md-sys-state-hover);
}

:global(.theme-dark) .repository-title {
  color: var(--md-sys-color-on-surface-variant);
}

:global(.theme-dark) .repository-title strong {
  color: var(--md-sys-color-on-surface);
}

:global(.theme-dark) .repository-title:hover,
:global(.dark) .repository-title:hover,
:global(.theme-dark) .repository-title:focus-visible,
:global(.dark) .repository-title:focus-visible {
  color: var(--md-sys-color-on-surface);
  border-color: var(--md-sys-color-outline);
  background: var(--md-sys-color-surface-container-high);
}

.fork-content {
  display: grid;
  grid-template-columns: 232px minmax(0, 1fr);
  min-height: 0;
  overflow: hidden;
  background: var(--md-sys-color-surface);
}

.shell-sidebar {
  min-height: 0;
  overflow: auto;
  padding: 8px;
  background: var(--md-sys-color-surface-container-low);
  border-right: 1px solid var(--md-sys-color-outline-variant);
}

.shell-repository {
  display: flex;
  align-items: center;
  min-height: 64px;
  gap: 12px;
  padding: 8px 10px 12px;
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  color: var(--md-sys-color-on-surface);
  font-size: 13px;
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
  font-size: 11px;
}

.sidebar-section {
  padding: 12px 2px 2px;
}

.sidebar-heading {
  padding: 3px 12px 7px;
  color: var(--md-sys-color-on-surface-variant);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: .06em;
}

.sidebar-section button {
  display: flex;
  align-items: center;
  width: 100%;
  height: 38px;
  gap: 10px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--app-radius-full);
  background: transparent;
  color: var(--md-sys-color-on-surface-variant);
  font-size: 13px;
  text-align: left;
}

.sidebar-section button:hover {
  color: var(--md-sys-color-on-surface);
  background: var(--md-sys-state-hover);
}

.sidebar-section button.active {
  color: var(--md-sys-color-on-secondary-container);
  background: var(--md-sys-color-secondary-container);
  box-shadow: none;
}

.sidebar-section button span {
  flex: 1;
}

.sidebar-section button b {
  font-size: 11px;
}

.route-workbench {
  display: grid;
  grid-template-rows: 48px minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  margin: 8px;
  overflow: hidden;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-lg);
  background: var(--md-sys-color-surface-container-lowest);
  box-shadow: var(--md-sys-elevation-1);
}

.route-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background: var(--md-sys-color-surface-container-lowest);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
  color: var(--md-sys-color-on-surface);
  font-size: 13px;
}

.route-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.route-accent {
  width: 4px;
  height: 20px;
  border-radius: var(--app-radius-full);
  background: var(--md-sys-color-primary);
}

.route-repository {
  color: var(--md-sys-color-on-surface-variant);
  font-size: 12px;
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
  display: block;
  height: 100%;
}

.route-content :deep(.empty-content) {
  margin: 42px 44px;
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
  padding: 8px 12px;
}

.route-content :deep(.el-card__body) {
  padding: 12px;
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
  font-size: 11px;
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
