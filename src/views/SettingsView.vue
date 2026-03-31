<template>
  <div class="settings-view">
    <el-card class="settings-card">
      <template #header>
        <span>{{ $t('nav.settings') }}</span>
      </template>

      <el-form :model="settings" label-width="150px" class="settings-form">
        <el-form-item :label="$t('language.label')">
          <el-select v-model="currentLanguage" style="width: 200px">
            <el-option
              v-for="lang in languages"
              :key="lang.value"
              :label="lang.label"
              :value="lang.value"
            />
          </el-select>
        </el-form-item>

        <el-form-item :label="$t('settings.svnPathPlaceholder')">
          <el-input
            v-model="settings.svnPath"
            :placeholder="$t('settings.svnPathPlaceholder')"
            clearable
          />
        </el-form-item>

        <el-form-item :label="$t('settings.encoding')">
          <el-select v-model="settings.encoding" style="width: 200px">
            <el-option label="UTF-8" value="utf-8" />
            <el-option label="GBK" value="gbk" />
            <el-option label="GB2312" value="gb2312" />
          </el-select>
        </el-form-item>

        <el-form-item :label="$t('settings.logLimit')">
          <el-input-number
            v-model="settings.logLimit"
            :min="1"
            :max="1000"
            :step="10"
          />
        </el-form-item>

        <el-form-item :label="$t('settings.autoRefresh')">
          <el-switch v-model="settings.autoRefresh" />
        </el-form-item>

        <el-form-item :label="$t('settings.theme')">
          <el-radio-group v-model="settings.theme">
            <el-radio label="light">{{ $t('settings.themeLight') }}</el-radio>
            <el-radio label="dark">{{ $t('settings.themeDark') }}</el-radio>
            <el-radio label="auto">{{ $t('settings.themeAuto') }}</el-radio>
          </el-radio-group>
        </el-form-item>

        <el-form-item>
          <el-button type="primary" @click="saveSettings">{{ $t('common.save') }}</el-button>
          <el-button @click="resetSettings">{{ $t('common.cancel') }}</el-button>
        </el-form-item>
      </el-form>

      <el-divider />

      <div class="about-section">
        <h3>{{ $t('nav.about') }}</h3>
        <p>OrcaSVN - 基于 Tauri 的 SVN 图形客户端</p>
        <p>{{ $t('common.version') }}：0.1.0</p>
        <p>{{ $t('common.techStack') }}：Tauri + Rust + Vue 3 + TypeScript</p>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLocale } from '@/composables/useLocale'

const { locale, t } = useI18n()
const { setLocale } = useLocale()

const languages = computed(() => [
  { label: t('language.zhCN'), value: 'zh-CN' },
  { label: t('language.jaJP'), value: 'ja-JP' },
  { label: t('language.enUS'), value: 'en-US' },
])

const settings = reactive({
  svnPath: '',
  encoding: 'utf-8',
  logLimit: 50,
  autoRefresh: true,
  theme: 'auto' as 'light' | 'dark' | 'auto',
})

// 使用计算属性获取和设置当前语言
const currentLanguage = computed({
  get: () => locale.value,
  set: (val: string) => setLocale(val)
})

const saveSettings = () => {
  setLocale(currentLanguage.value)
  // TODO: 保存到本地存储
  console.log('保存设置:', settings, '语言:', currentLanguage.value)
}

const resetSettings = () => {
  settings.svnPath = ''
  settings.encoding = 'utf-8'
  settings.logLimit = 50
  settings.autoRefresh = true
  settings.theme = 'auto'
  setLocale('zh-CN')
}
</script>

<style scoped>
.settings-view {
  max-width: 700px;
  margin: 0 auto;
}

.settings-form {
  margin-top: 20px;
}

.about-section {
  padding: 20px 0;
}

.about-section h3 {
  margin-bottom: 15px;
}

.about-section p {
  margin: 8px 0;
  color: #666;
}
</style>
