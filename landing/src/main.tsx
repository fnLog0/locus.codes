import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
/* Oat: include before our styles so we can override (https://oat.ink/usage/) */
import '@knadh/oat/oat.min.css'
import '@knadh/oat/oat.min.js'
import './oat-theme.css'
import './css/main.css'
import App from './App'

const root = document.getElementById('root')
if (!root) throw new Error('Root element not found')

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
