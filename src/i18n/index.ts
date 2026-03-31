import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN'
import jaJP from './locales/ja-JP'
import enUS from './locales/en-US'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'en-US',
  messages: {
    'zh-CN': zhCN,
    'ja-JP': jaJP,
    'en-US': enUS,
  },
})

export default i18n
