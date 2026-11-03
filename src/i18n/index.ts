import { watch } from 'vue'
import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN'
import zhTW from './locales/zh-TW'
import jaJP from './locales/ja-JP'
import koKR from './locales/ko-KR'
import enUS from './locales/en-US'

export const supportedLocales = ['zh-CN', 'zh-TW', 'ja-JP', 'ko-KR', 'en-US'] as const
export type SupportedLocale = typeof supportedLocales[number]

export function detectSystemLocale(): SupportedLocale {
  const languages = navigator.languages?.length ? navigator.languages : [navigator.language]

  for (const language of languages) {
    const normalized = language.toLowerCase()
    if (normalized === 'zh-tw' || normalized === 'zh-hk' || normalized === 'zh-mo' || normalized === 'zh-hant') {
      return 'zh-TW'
    }
    if (normalized === 'zh-cn' || normalized === 'zh-sg' || normalized === 'zh-hans' || normalized === 'zh') {
      return 'zh-CN'
    }
    if (normalized.startsWith('ja')) return 'ja-JP'
    if (normalized.startsWith('ko')) return 'ko-KR'
    if (normalized.startsWith('en')) return 'en-US'
  }

  return 'en-US'
}

function getStoredLocale(): string {
  try {
    const stored = localStorage.getItem('orcasvn-settings')
    if (stored) {
      const settings = JSON.parse(stored)
      if (supportedLocales.includes(settings.language)) return settings.language
    }
  } catch {
    // ignore
  }
  return detectSystemLocale()
}

const i18n = createI18n({
  legacy: false,
  locale: getStoredLocale(),
  fallbackLocale: 'en-US',
  messages: {
    'zh-CN': zhCN,
    'zh-TW': zhTW,
    'ja-JP': jaJP,
    'ko-KR': koKR,
    'en-US': enUS,
  },
})

function syncDocumentLang(locale: string) {
  document.documentElement.lang = locale
}

syncDocumentLang(i18n.global.locale.value)
watch(i18n.global.locale, syncDocumentLang)

export default i18n
