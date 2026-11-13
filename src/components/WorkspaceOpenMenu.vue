<template>
  <el-dropdown trigger="click" :disabled="!path" @command="handleCommand">
    <button type="button" class="open-directory" :class="{ compact }" :disabled="!path" :aria-label="t('menu.openIn')" :title="t('menu.openIn')" @click.stop>
      <el-icon><FolderOpened /></el-icon>
      <span v-if="!compact">{{ t('menu.openIn') }}</span>
      <el-icon v-if="!compact"><ArrowDown /></el-icon>
    </button>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item v-for="target in openTargets" :key="target" :command="target" :class="{ 'selected-target': selectedTarget === target }">
          {{ t(`openIn.${target}`) }}
        </el-dropdown-item>
        <el-dropdown-item command="configure" divided>{{ t('openIn.configure') }}</el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import { openTargets, useWorkspaceOpener } from '@/composables/useWorkspaceOpener'
import type { OpenWorkspaceTarget } from '@/api/svn'

const props = defineProps<{ path: string | null; compact?: boolean }>()
const { t } = useI18n()
const { openIn, selectedTarget, settingsVisible } = useWorkspaceOpener()
async function handleCommand(command: OpenWorkspaceTarget | 'configure') {
  if (command === 'configure') { settingsVisible.value = true; return }
  try { await openIn(props.path || '', command) }
  catch (error) { ElMessage.error(String(error)) }
}
</script>

<style scoped>
.open-directory { display: flex; align-items: center; gap: 6px; height: 36px; padding: 0 10px; border: 0; border-radius: 6px; background: transparent; color: var(--md-sys-color-on-surface); font: inherit; font-size: 12px; cursor: pointer; }
.open-directory:hover { background: var(--md-sys-state-hover); }
.open-directory:focus-visible { outline: 2px solid var(--md-sys-color-primary); }
.open-directory:disabled { opacity: .5; cursor: default; }
.compact { width: 24px; height: 24px; justify-content: center; padding: 0; }
.selected-target { color: var(--md-sys-color-primary); font-weight: 600; }
</style>
