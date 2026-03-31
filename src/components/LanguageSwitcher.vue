<template>
  <el-dropdown @command="handleLanguageChange">
    <el-button text>
      <el-icon><Globe /></el-icon>
      {{ currentLanguageLabel }}
      <el-icon class="el-icon--right"><ArrowDown /></el-icon>
    </el-button>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="lang in languages"
          :key="lang.value"
          :command="lang.value"
        >
          {{ lang.label }}
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useLocale } from '@/composables/useLocale'

const { locale, t } = useI18n()
const { setLocale } = useLocale()

const languages = computed(() => [
  { label: t('language.zhCN'), value: 'zh-CN' },
  { label: t('language.jaJP'), value: 'ja-JP' },
  { label: t('language.enUS'), value: 'en-US' },
])

const currentLanguageLabel = computed(() => {
  const lang = languages.value.find((l) => l.value === locale.value)
  return lang?.label || t('language.zhCN')
})

const handleLanguageChange = (command: string) => {
  setLocale(command)
}
</script>
