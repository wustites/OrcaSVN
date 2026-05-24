<template>
  <div class="settings-view">
    <el-card class="settings-card animate-fade-in">
      <template #header>
        <div class="card-header">
          <span class="card-title">
            <el-icon><Setting /></el-icon>
            {{ $t('nav.settings') }}
          </span>
        </div>
      </template>

      <el-form :model="settings" label-width="150px" class="settings-form" label-position="left">
        <div class="settings-section">
          <h3 class="section-title">
            <el-icon><Monitor /></el-icon>
            {{ $t('settings.appearance') }}
          </h3>
          
          <el-form-item :label="$t('language.label')" class="form-item">
            <el-select v-model="currentLanguage" style="width: 200px" class="language-select">
              <el-option
                v-for="lang in languages"
                :key="lang.value"
                :label="lang.label"
                :value="lang.value"
              />
            </el-select>
          </el-form-item>

          <el-form-item :label="$t('settings.theme')" class="form-item">
            <el-radio-group v-model="settings.theme" @change="applyTheme" class="theme-group">
              <el-radio label="light" class="theme-radio">
                <div class="theme-option">
                  <el-icon><Sunny /></el-icon>
                  <span>{{ $t('settings.themeLight') }}</span>
                </div>
              </el-radio>
              <el-radio label="dark" class="theme-radio">
                <div class="theme-option">
                  <el-icon><Moon /></el-icon>
                  <span>{{ $t('settings.themeDark') }}</span>
                </div>
              </el-radio>
              <el-radio label="auto" class="theme-radio">
                <div class="theme-option">
                  <el-icon><Monitor /></el-icon>
                  <span>{{ $t('settings.themeAuto') }}</span>
                </div>
              </el-radio>
            </el-radio-group>
          </el-form-item>
        </div>

        <el-divider />

        <div class="settings-section">
          <h3 class="section-title">
            <el-icon><FolderOpened /></el-icon>
            {{ $t('settings.svnSettings') }}
          </h3>

          <el-form-item :label="$t('settings.svnPathPlaceholder')" class="form-item">
            <el-input
              v-model="settings.svnPath"
              :placeholder="$t('settings.svnPathPlaceholder')"
              clearable
            >
              <template #prefix>
                <el-icon><Folder /></el-icon>
              </template>
            </el-input>
          </el-form-item>

          <el-form-item :label="$t('settings.encoding')" class="form-item">
            <el-select v-model="settings.encoding" style="width: 200px">
              <el-option label="UTF-8" value="utf-8" />
              <el-option label="GBK" value="gbk" />
              <el-option label="GB2312" value="gb2312" />
            </el-select>
          </el-form-item>

          <el-form-item :label="$t('settings.logLimit')" class="form-item">
            <el-input-number
              v-model="settings.logLimit"
              :min="1"
              :max="1000"
              :step="10"
              controls-position="right"
            />
          </el-form-item>

          <el-form-item :label="$t('settings.autoRefresh')" class="form-item">
            <el-switch v-model="settings.autoRefresh" />
          </el-form-item>
        </div>

        <el-divider />

        <el-form-item class="form-actions">
          <el-button type="primary" @click="handleSave">
            <el-icon><Check /></el-icon>
            {{ $t('common.save') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><RefreshLeft /></el-icon>
            {{ $t('common.cancel') }}
          </el-button>
        </el-form-item>
      </el-form>

      <el-divider />

      <div class="about-section">
        <h3 class="section-title">
          <el-icon><InfoFilled /></el-icon>
          {{ $t('nav.about') }}
        </h3>
        <div class="about-content">
          <div class="about-logo">
            <div class="logo-mark">
              <el-icon><FolderOpened /></el-icon>
            </div>
            <div class="about-info">
              <h4>OrcaSVN</h4>
              <p class="about-subtitle">SVN Workbench</p>
            </div>
          </div>
          <div class="about-details">
            <div class="detail-item">
              <span class="detail-label">{{ $t('common.version') }}：</span>
              <el-tag size="small" type="primary">0.2.0</el-tag>
            </div>
            <div class="detail-item">
              <span class="detail-label">{{ $t('common.techStack') }}：</span>
              <span class="detail-value">Tauri + Rust + Vue 3 + TypeScript</span>
            </div>
          </div>
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLocale } from '@/composables/useLocale'
import { useSettings } from '@/composables/useSettings'

const { t } = useI18n()
const { setLocale } = useLocale()
const { settings, resetSettings } = useSettings()

const languages = computed(() => [
  { label: t('language.zhCN'), value: 'zh-CN' },
  { label: t('language.zhTW'), value: 'zh-TW' },
  { label: t('language.jaJP'), value: 'ja-JP' },
  { label: t('language.koKR'), value: 'ko-KR' },
  { label: t('language.enUS'), value: 'en-US' },
])

const currentLanguage = computed({
  get: () => settings.language,
  set: (val: string) => {
    settings.language = val
    setLocale(val)
  },
})

function applyTheme(theme: string) {
  const root = document.documentElement
  root.classList.remove('theme-light', 'theme-dark')

  if (theme === 'dark') {
    root.classList.add('theme-dark')
  } else if (theme === 'light') {
    root.classList.add('theme-light')
  } else {
    // auto: follow system
    if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
      root.classList.add('theme-dark')
    }
  }
}

function handleSave() {
  setLocale(settings.language)
  applyTheme(settings.theme)
}

function handleReset() {
  resetSettings()
  setLocale(settings.language)
  applyTheme(settings.theme)
}

onMounted(() => {
  setLocale(settings.language)
  applyTheme(settings.theme)
})
</script>

<style scoped>
.settings-view {
  max-width: 700px;
  margin: 0 auto;
}

.settings-card {
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

.settings-form {
  margin-top: var(--app-spacing-md);
}

.settings-section {
  margin-bottom: var(--app-spacing-md);
}

.section-title {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  margin-bottom: var(--app-spacing-lg);
  font-size: 16px;
  font-weight: 700;
  color: var(--el-text-color-primary);
}

.form-item {
  margin-bottom: var(--app-spacing-md);
}

.theme-group {
  display: flex;
  gap: var(--app-spacing);
}

.theme-radio {
  margin-right: 0;
}

.theme-option {
  display: flex;
  align-items: center;
  gap: 6px;
}

.theme-option .el-icon {
  font-size: 16px;
}

.form-actions {
  margin-bottom: 0;
}

.about-section {
  padding: var(--app-spacing-md) 0;
}

.about-content {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-lg);
  margin-top: var(--app-spacing-md);
}

.about-logo {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-md);
}

.logo-mark {
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border-radius: var(--app-radius);
  background: linear-gradient(135deg, var(--md-sys-color-primary-container), var(--md-sys-color-secondary-container));
  color: var(--md-sys-color-primary);
  font-size: 24px;
  box-shadow: var(--md-sys-elevation-1);
}

.about-info h4 {
  margin: 0;
  font-size: 18px;
  font-weight: 800;
  color: var(--el-text-color-primary);
}

.about-subtitle {
  margin: 2px 0 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.about-details {
  display: flex;
  flex-direction: column;
  gap: var(--app-spacing-sm);
}

.detail-item {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
}

.detail-label {
  font-weight: 600;
  color: var(--el-text-color-regular);
}

.detail-value {
  color: var(--el-text-color-primary);
}

@media (max-width: 640px) {
  .settings-view {
    padding: 0 var(--app-spacing);
  }
  
  .settings-form :deep(.el-form-item) {
    flex-direction: column;
  }
  
  .settings-form :deep(.el-form-item__label) {
    text-align: left;
    padding-bottom: 4px;
  }
  
  .theme-group {
    flex-direction: column;
  }
}
</style>
