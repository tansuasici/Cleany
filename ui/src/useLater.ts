import { useEffect, useState } from 'react'

/** True once `key` has stayed the same, and not null, for `ms`. */
export function useLater(key: string | null, ms: number): boolean {
  const [due, setDue] = useState<string | null>(null)
  useEffect(() => {
    if (key === null) return
    const id = window.setTimeout(() => setDue(key), ms)
    return () => window.clearTimeout(id)
  }, [key, ms])
  return key !== null && due === key
}
