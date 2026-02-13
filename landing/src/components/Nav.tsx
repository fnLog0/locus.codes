import { ThemeSwitch } from './ThemeSwitch'

export interface NavProps {
  isDark: boolean
  onThemeToggle: () => void
}

export function Nav({ isDark, onThemeToggle }: NavProps) {
  return (
    <header className="vp-nav">
      <a href="/" className="vp-nav-logo">
        locus.codes
      </a>
      <div className="vp-nav-links">
        <ThemeSwitch isDark={isDark} onToggle={onThemeToggle} />
      </div>
    </header>
  )
}
