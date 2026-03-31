import { useI18n } from 'vue-i18n'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
import ja from 'element-plus/dist/locale/ja.mjs'
import en from 'element-plus/dist/locale/en.mjs'
import { computed } from 'vue'

const localeMap: Record<string, any> = {
  'zh-CN': zhCn,
  'ja-JP': ja,
  'en-US': en,
}

export function useLocale() {
  const { locale } = useI18n()

  const elementLocale = computed(() => {
    return localeMap[locale.value] || zhCn
  })

  const setLocale = (newLocale: string) => {
    locale.value = newLocale
  }

  return {
    locale,
    elementLocale,
    setLocale,
  }
}
