import { useEffect, useState } from 'react'
import { type AppInfo, getAppInfo } from './lib/ipc'

// Placeholder shell for M0: proves the Rust <-> UI bridge works.
// Real screens follow the approved DESIGN_MAP direction (M2).
export default function App() {
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    getAppInfo().then(setInfo).catch((e: unknown) => setError(String(e)))
  }, [])

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-2 p-6">
      <h1 className="text-2xl font-semibold">Cleany</h1>
      {info && (
        <p className="text-sm opacity-70">
          v{info.version} · {info.os}/{info.arch}
        </p>
      )}
      {error && <p className="text-sm text-red-600">{error}</p>}
    </main>
  )
}
