<template>
  <div class="checkout-view">
    <el-card class="checkout-card">
      <template #header>
        <span>{{ $t('checkout.title') }}</span>
      </template>

      <el-form :model="form" label-width="100px" class="checkout-form">
        <el-form-item :label="$t('checkout.repositoryUrl')" required>
          <el-input
            v-model="form.url"
            :placeholder="$t('checkout.repositoryUrlPlaceholder')"
            clearable
          >
            <template #prepend>
              <el-icon><Link /></el-icon>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item :label="$t('checkout.targetDirectory')" required>
          <el-input
            v-model="form.path"
            :placeholder="$t('checkout.selectTargetDirectory')"
            clearable
          >
            <template #append>
              <el-button @click="selectDirectory">
                <el-icon><Folder /></el-icon>
                {{ $t('common.browse') }}
              </el-button>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item :label="$t('checkout.revision')">
          <el-input
            v-model.number="form.revision"
            type="number"
            :placeholder="$t('checkout.revisionPlaceholder')"
            clearable
          />
        </el-form-item>

        <el-form-item>
          <el-button type="primary" @click="doCheckout" :loading="loading">
            <el-icon><Download /></el-icon>
            {{ $t('checkout.startCheckout') }}
          </el-button>
          <el-button @click="resetForm">{{ $t('common.reset') }}</el-button>
        </el-form-item>
      </el-form>

      <div v-if="output" class="output-area">
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
    title: '选择目标目录'
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

    // 设置当前工作区
    workspaceStore.setCurrentPath(form.path)

    // 跳转到工作区
    setTimeout(() => {
      router.push({ name: 'workspace' })
    }, 1000)
  } catch (err) {
    output.value = `错误：${err}`
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

.checkout-form {
  margin-top: 20px;
}

.output-area {
  margin-top: 20px;
}

.output-textarea {
  font-family: 'Consolas', 'Monaco', monospace;
}
</style>
