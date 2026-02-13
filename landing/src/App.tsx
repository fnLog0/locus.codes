import { useTheme } from './hooks/useTheme'
import { PageBackground, Nav, Hero, Description, GetCTA } from './components'

export default function App() {
  const { isDark, toggleTheme } = useTheme()

  return (
    <>
      <PageBackground />
      <Nav isDark={isDark} onThemeToggle={toggleTheme} />
      <main>
        <Hero />
        <Description />
        <GetCTA />
      </main>
    </>
  )
}
