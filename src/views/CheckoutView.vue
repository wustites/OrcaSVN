<template>
  <div class="checkout-view">
    <el-card class="checkout-card animate-fade-in">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon><Download /></el-icon>
            {{ $t('checkout.title') }}
          </span>
        </div>
      </template>

      <el-form :model="form" label-width="120px" class="checkout-form" label-position="left">
        <el-form-item :label="$t('checkout.repositoryUrl')" required class="form-item">
          <el-input
            v-model="form.url"
            :placeholder="$t('checkout.repositoryUrlPlaceholder')"
            clearable
          >
            <template #prefix>
              <el-icon><Link /></el-icon>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item :label="$t('checkout.targetDirectory')" required class="form-item">
          <el-input
            v-model="form.path"
            :placeholder="$t('checkout.selectTargetDirectory')"
            clearable
          >
            <template #prefix>
              <el-icon><Folder /></el-icon>
            </template>
            <template #append>
              <el-button @click="selectDirectory">
                <el-icon><FolderOpened /></el-icon>
                {{ $t('common.browse') }}
              </el-button>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item :label="$t('checkout.revision')" class="form-item">
          <el-input
            v-model.number="form.revision"
            type="number"
            :placeholder="$t('checkout.revisionPlaceholder')"
            clearable
          >
            <template #prefix>
              <el-icon><PriceTag /></el-icon>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item class="form-actions">
          <el-button type="primary" @click="doCheckout" :loading="loading" size="large">
            <el-icon><Download /></el-icon>
            {{ $t('checkout.startCheckout') }}
          </el-button>
          <el-button @click="resetForm" size="large">
            <el-icon><RefreshLeft /></el-icon>
            {{ $t('common.reset') }}
          </el-button>
        </el-form-item>
      </el-form>

      <div v-if="output" class="output-area animate-fade-in">
        <div class="output-header">
          <span class="output-title">
            <el-icon><Document /></el-icon>
            {{ $t('checkout.output') }}
          </span>
          <el-tag :type="output.includes('Error') ? 'danger' : 'success'" size="small">
            {{ output.includes('Error') ? $t('common.error') : $t('common.success') }}
          </el-tag>
        </div>
        <el-input
          v-model="output"
          type="textarea"
          :rows="10"
          readonly
          class="output-textarea"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '@/stores/workspace'
import { svnCheckout } from '@/api/svn'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const router = useRouter()
const workspaceStore = useWorkspaceStore()

const form = reactive({
  url: '',
  path: '',
  revision: undefined as number | undefined
})

const loading = ref(false)
const output = ref('')

const selectDirectory = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t('dialog.selectTargetDirectory')
  })

  if (selected) {
    form.path = Array.isArray(selected) ? selected[0] : selected
  }
}

const doCheckout = async () => {
  if (!form.url || !form.path) {
    return
  }

  loading.value = true
  output.value = ''

  try {
    const result = await svnCheckout(form.url, form.path, form.revision)
    output.value = result.output

    workspaceStore.setCurrentPath(form.path)

    setTimeout(() => {
      router.push({ name: 'workspace' })
    }, 1500)
  } catch (err) {
    output.value = `${t('common.error')}：${err}`
  } finally {
    loading.value = false
  }
}

const resetForm = () => {
  form.url = ''
  form.path = ''
  form.revision = undefined
  output.value = ''
}
</script>

<style scoped>
.checkout-view {
  max-width: 800px;
  margin: 0 auto;
}

.checkout-card {
  border-radius: var(--app-radius-lg);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-title {
  display: inline-flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  font-weight: 700;
}

.checkout-form {
  margin-top: var(--app-spacing-md);
}

.form-item {
  margin-bottom: var(--app-spacing-lg);
}

.form-actions {
  margin-bottom: 0;
  margin-top: var(--app-spacing-lg);
}

.output-area {
  margin-top: var(--app-spacing-lg);
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-md);
  overflow: hidden;
}

.output-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--app-spacing) var(--app-spacing-md);
  background: var(--el-fill-color-light);
  border-bottom: 1px solid var(--md-sys-color-outline-variant);
}

.output-title {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  font-size: 13px;
  color: var(--el-text-color-regular);
}

.output-textarea {
  font-family: "Cascadia Mono", Consolas, Monaco, monospace;
}

:deep(.output-textarea .el-textarea__inner) {
  border: none;
  border-radius: 0;
  padding: var(--app-spacing-md);
}

@media (max-width: 640px) {
  .checkout-view {
    padding: 0 var(--app-spacing);
  }
  
  .checkout-form :deep(.el-form-item) {
    flex-direction: column;
  }
  
  .checkout-form :deep(.el-form-item__label) {
    text-align: left;
    padding-bottom: 4px;
  }
  
  .form-actions {
    display: flex;
    flex-direction: column;
  }
  
  .form-actions .el-button {
    width: 100%;
  }
}
</style>
