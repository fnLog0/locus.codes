import { useState, useEffect } from 'react'

const THEME_KEY = 'locuscodes-theme'

type ThemePreference = 'light' | 'dark' | 'system'

function getSystemDark(): boolean {
  if (typeof window === 'undefined') return false
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

function getStoredPreference(): ThemePreference {
  if (typeof window === 'undefined') return 'light'
  const stored = localStorage.getItem(THEME_KEY)
  return (stored === 'light' || stored === 'dark' || stored === 'system') ? stored : 'light'
}

function applyTheme(dark: boolean): void {
  if (typeof document === 'undefined') return
  document.body.setAttribute('data-theme', dark ? 'dark' : '')
}

export function useTheme() {
  const [pref, setPref] = useState<ThemePreference>(getStoredPreference)
  const [systemDark, setSystemDark] = useState(getSystemDark)
  const isDark = pref === 'system' ? systemDark : pref === 'dark'

  useEffect(() => {
    applyTheme(isDark)
  }, [isDark])

  useEffect(() => {
    const m = window.matchMedia('(prefers-color-scheme: dark)')
    const handle = () => setSystemDark(m.matches)
    m.addEventListener('change', handle)
    return () => m.removeEventListener('change', handle)
  }, [])

  const setTheme = (next: ThemePreference) => {
    setPref(next)
    localStorage.setItem(THEME_KEY, next)
  }

  const toggleTheme = () => {
    setTheme(isDark ? 'light' : 'dark')
  }

  return { isDark, setTheme, toggleTheme }
}
