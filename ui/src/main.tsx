import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

// The webview's own right-click menu (Reload, Inspect Element, Services) is not
// part of the app. Text fields keep theirs for cut, copy and paste. In a debug
// build the inspector still opens with ⌥⌘I.
document.addEventListener('contextmenu', (e) => {
  const el = e.target instanceof Element ? e.target : null
  if (el?.closest('input, textarea, [contenteditable="true"]')) return
  e.preventDefault()
})

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
