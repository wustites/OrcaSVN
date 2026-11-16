<template>
  <el-dialog v-model="settingsVisible" :title="t('openIn.configure')" width="min(640px, 92vw)" append-to-body @open="resetDraft">
    <p class="editor-help">{{ t('openIn.pathHelp') }}</p>
    <el-form label-position="top" @submit.prevent="save">
      <el-form-item v-for="target in editorTargets" :key="target" :label="t(`openIn.${target}`)">
        <div class="editor-path">
          <el-input v-model="draft[target]" :aria-label="t(`openIn.${target}`)" :placeholder="t(target === 'custom' ? 'openIn.customPlaceholder' : 'openIn.autoDetect')" clearable />
          <el-button @click="browse(target)">{{ t('openIn.browse') }}</el-button>
        </div>
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="settingsVisible = false">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" @click="save">{{ t('common.save') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { ElMessage } from 'element-plus/es/components/message/index'
import { editorTargets, useWorkspaceOpener, type EditorTarget } from '@/composables/useWorkspaceOpener'

const { t } = useI18n()
const { editorPaths, settingsVisible, savePaths } = useWorkspaceOpener()
const draft = reactive({ ...editorPaths })
function resetDraft() { Object.assign(draft, editorPaths) }
async function browse(target: EditorTarget) {
  try {
    const selected = await open({ title: `${t('openIn.browse')} · ${t(`openIn.${target}`)}`, multiple: false, directory: false })
    if (typeof selected === 'string') draft[target] = selected
  } catch (error) { ElMessage.error(String(error)) }
}
function save() {
  try { savePaths(draft); settingsVisible.value = false }
  catch (error) { ElMessage.error(`${t('openIn.saveFailed')}：${String(error)}`) }
}
</script>

<style scoped>
.editor-help { margin-top: 0; line-height: 1.6; color: var(--md-sys-color-on-surface-variant); }
.editor-path { display: flex; gap: 8px; width: 100%; }
.editor-path .el-input { flex: 1; min-width: 0; }
</style>
