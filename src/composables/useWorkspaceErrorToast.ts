import { watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus/es/components/message/index'
import { useWorkspaceStore } from '@/stores/workspace'

export function useWorkspaceErrorToast() {
  const workspaceStore = useWorkspaceStore()
  const { t } = useI18n()

  watch(() => workspaceStore.error, (error, _previousError, onCleanup) => {
    if (!error) return
    let dismissedByUser = true
    const message = ElMessage.error({
      message: `${t('common.error')}：${error}`,
      duration: 0,
      showClose: true,
      // Clear the store so the same error can be shown again if it recurs.
      onClose: () => {
        if (dismissedByUser) workspaceStore.setError(null)
      },
    })
    onCleanup(() => {
      dismissedByUser = false
      message.close()
    })
  })
}
