<template>
  <div class="settings-view">
    <el-card class="settings-card animate-fade-in">
      <el-form :model="settings" label-width="180px" class="settings-form" label-position="left">
        <div class="settings-section">
          <h3 class="section-title">
            <el-icon><Monitor /></el-icon>
            {{ $t('settings.appearance') }}
          </h3>
          
          <el-form-item :label="$t('language.label')" class="form-item">
            <el-select
              v-model="currentLanguage"
              class="settings-control language-select"
              popper-class="settings-select-dropdown"
            >
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

          <el-form-item :label="$t('settings.svnPath')" class="form-item">
            <el-input
              v-model="settings.svnPath"
              :placeholder="$t('settings.svnPathPlaceholder')"
              class="settings-control settings-path-input"
              clearable
              @change="handleSvnPathChange"
            >
              <template #prefix>
                <el-icon><Folder /></el-icon>
              </template>
            </el-input>
          </el-form-item>

          <el-form-item :label="$t('settings.encoding')" class="form-item">
            <el-select
              v-model="settings.encoding"
              class="settings-control"
              popper-class="settings-select-dropdown"
            >
              <el-option label="UTF-8" value="utf-8" />
              <el-option label="GBK" value="gbk" />
              <el-option label="GB2312" value="gb2312" />
            </el-select>
          </el-form-item>

          <el-form-item :label="$t('settings.gitignore')" class="form-item">
            <el-switch v-model="settings.gitignoreEnabled" @change="handleGitignoreChange" />
            <span class="form-item-hint">{{ $t('settings.gitignoreHint') }}</span>
          </el-form-item>

          <el-form-item :label="$t('settings.autoDeleteMissingOnCommit')" class="form-item">
            <el-switch v-model="settings.autoDeleteMissingOnCommit" />
            <span class="form-item-hint">{{ $t('settings.autoDeleteMissingOnCommitHint') }}</span>
          </el-form-item>
        </div>
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
              <p class="about-subtitle">{{ $t('settings.aboutSubtitle') }}</p>
            </div>
          </div>
          <div class="about-details">
            <div class="detail-item">
              <span class="detail-label">{{ $t('common.version') }}：</span>
              <el-tag size="small" type="primary">{{ appVersion }}</el-tag>
              <el-button
                v-if="updateState === 'available'"
                size="small"
                type="primary"
                link
                @click="openReleasePage"
              >
                {{ $t('appUpdate.available', { version: latestVersion }) }}
              </el-button>
              <span v-else-if="updateState !== 'idle'" class="update-status">
                {{ $t(`appUpdate.${updateState}`) }}
              </span>
              <el-button
                v-if="updateState !== 'checking'"
                size="small"
                link
                @click="checkForUpdates"
              >
                {{ $t('appUpdate.check') }}
              </el-button>
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
import { ElMessage } from 'element-plus/es/components/message/index'
import { configureSvnExecutable } from '@/api/svn'
import { useLocale } from '@/composables/useLocale'
import { useSettings } from '@/composables/useSettings'
import { applyTheme } from '@/composables/useTheme'
import { useWorkspace } from '@/composables/useWorkspace'
import { useUpdateCheck } from '@/composables/useUpdateCheck'

const { t } = useI18n()
const { setLocale } = useLocale()
const { settings } = useSettings()
const { refreshStatusAfterMutation } = useWorkspace()
const { currentVersion: appVersion, state: updateState, latestVersion, checkForUpdates, openReleasePage } = useUpdateCheck()

const languages = computed(() => [
  { label: t('language.zhCN'), value: 'zh-CN' },
  { label: t('language.zhTW'), value: 'zh-TW' },
  { label: t('language.jaJP'), value: 'ja-JP' },
  { label: t('language.koKR'), value: 'ko-KR' },
  { label: t('language.enUS'), value: 'en-US' },
])

const currentLanguage = computed({
  get: () => settings.language,
  set: (val: string) => setLocale(val),
})

const handleGitignoreChange = () => {
  void refreshStatusAfterMutation()
}

const handleSvnPathChange = async (value: string) => {
  try {
    const version = await configureSvnExecutable(value)
    ElMessage.success(t('settings.svnPathApplied', { version }))
  } catch (err) {
    ElMessage.error(t('settings.svnPathInvalid', { error: String(err) }))
  }
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

.settings-form {
  margin-top: 0;
}

.settings-section {
  margin-bottom: var(--app-spacing-md);
}

.section-title {
  display: flex;
  align-items: center;
  gap: var(--app-spacing-sm);
  margin-bottom: var(--app-spacing-lg);
  font-size: var(--app-font-size-subtitle);
  font-weight: var(--app-font-weight-bold);
  color: var(--el-text-color-primary);
}

.form-item {
  margin-bottom: var(--app-spacing-md);
}

.settings-form :deep(.el-form-item__label) {
  align-items: center;
  justify-content: flex-start;
  min-height: var(--app-size-field);
  padding-right: var(--app-spacing-md);
  line-height: 1.35;
  white-space: nowrap;
  word-break: keep-all;
}

.settings-form :deep(.el-form-item__content) {
  min-width: 0;
}

.settings-control {
  width: 260px;
  max-width: 100%;
}

.form-item-hint {
  margin-left: var(--app-spacing-sm);
  font-size: var(--app-font-size-sm);
  color: var(--el-text-color-secondary);
}

.settings-path-input {
  width: 260px;
}

.theme-group {
  display: flex;
  flex-wrap: wrap;
  gap: var(--app-spacing-sm);
}

.theme-radio {
  margin-right: 0;
  min-height: var(--app-size-control-compact);
  padding: 0 10px;
  border: 1px solid var(--md-sys-color-outline-variant);
  border-radius: var(--app-radius-sm);
  background: var(--md-sys-color-surface-container-low);
  transition: background-color var(--app-transition-fast), border-color var(--app-transition-fast);
}

.theme-radio:hover {
  border-color: var(--md-sys-color-outline);
  background: var(--md-sys-color-surface-container-high);
}

.theme-radio.is-checked {
  border-color: var(--md-sys-color-primary);
  background: var(--md-sys-color-primary-container);
}

.theme-option {
  display: flex;
  align-items: center;
  gap: 6px;
}

.theme-option .el-icon {
  font-size: var(--app-size-icon-sm);
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
  font-size: var(--app-font-size-title);
  font-weight: 800;
  color: var(--el-text-color-primary);
}

.about-subtitle {
  margin: 2px 0 0;
  color: var(--el-text-color-secondary);
  font-size: var(--app-font-size-label);
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

.update-status {
  color: var(--el-text-color-secondary);
  font-size: var(--app-font-size-xs);
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

  .settings-control,
  .settings-path-input {
    width: 100%;
  }
}
</style>
