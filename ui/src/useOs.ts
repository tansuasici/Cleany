import { useEffect, useState } from 'react'
import { getAppInfo } from './lib/ipc'

export type Os = 'macos' | 'windows' | 'other'
let osOnce: Promise<Os> | null = null
/** The OS, asked once per app run; it names the file manager. */
export function useOs(): Os | null {
  const [os, setOs] = useState<Os | null>(null)
  useEffect(() => {
    let live = true
    osOnce ??= getAppInfo().then(
      (i) => (i.os === 'macos' || i.os === 'windows' ? i.os : 'other'),
      () => 'other' as const,
    )
    void osOnce.then((o) => live && setOs(o))
    return () => {
      live = false
    }
  }, [])
  return os
}
