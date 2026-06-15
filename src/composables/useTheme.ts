import { watch } from 'vue'
import { useSettings, type AppSettings } from '@/composables/useSettings'

const systemTheme = window.matchMedia('(prefers-color-scheme: dark)')
let initialized = false

export function applyTheme(theme: AppSettings['theme']) {
  const isDark = theme === 'dark' || (theme === 'auto' && systemTheme.matches)
  document.documentElement.classList.toggle('theme-dark', isDark)
  document.documentElement.classList.toggle('dark', isDark)
  document.documentElement.classList.toggle('theme-light', !isDark)
  document.documentElement.style.colorScheme = isDark ? 'dark' : 'light'
}

export function initializeTheme() {
  if (initialized) return
  initialized = true

  const { settings } = useSettings()
  applyTheme(settings.theme)

  watch(
    () => settings.theme,
    theme => applyTheme(theme),
    { flush: 'sync' },
  )

  systemTheme.addEventListener('change', () => {
    if (settings.theme === 'auto') applyTheme('auto')
  })
}
