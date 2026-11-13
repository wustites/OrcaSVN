import { reactive, ref } from 'vue'
import { openWorkspaceTarget, type OpenWorkspaceTarget } from '@/api/svn'

export const editorTargets = ['android-studio', 'idea', 'vscode', 'custom'] as const
export const openTargets: OpenWorkspaceTarget[] = ['explorer', ...editorTargets, 'terminal']
export type EditorTarget = typeof editorTargets[number]
const storageKey = 'orcasvn-open-programs-v1'
const editorPaths = reactive<Record<EditorTarget, string>>({ 'android-studio': '', idea: '', vscode: '', custom: '' })
const selectedTarget = ref<OpenWorkspaceTarget>('explorer')
const settingsVisible = ref(false)

try {
  const stored = JSON.parse(localStorage.getItem(storageKey) || '{}')
  for (const target of editorTargets) {
    if (typeof stored?.[target] === 'string') editorPaths[target] = stored[target]
  }
} catch { /* 忽略无效配置，编辑器仍可自动查找。 */ }

export function useWorkspaceOpener() {
  function savePaths(paths: Record<EditorTarget, string>) {
    const next = Object.fromEntries(editorTargets.map(target => [target, paths[target].trim()])) as Record<EditorTarget, string>
    localStorage.setItem(storageKey, JSON.stringify(next))
    Object.assign(editorPaths, next)
  }

  async function openIn(path: string, target: OpenWorkspaceTarget) {
    if (!path) return
    await openWorkspaceTarget(path, target, editorPaths[target as EditorTarget])
    selectedTarget.value = target
  }

  return { editorPaths, selectedTarget, settingsVisible, savePaths, openIn }
}
