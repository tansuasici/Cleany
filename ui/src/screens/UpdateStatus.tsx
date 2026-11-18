import { useEffect, useRef, useState } from 'react'
import { Button } from '../components/ui'
import { formatBytes, formatShare } from '../lib/format'
import { checkUpdate, getAppInfo, installUpdate, type UpdateInfo } from '../lib/ipc'
import { useT } from '../useT'

/** Sent by the update item in the app menu and the menu bar menu (src-tauri/src/update.rs). */
const CHECK_EVENT = 'cleany:check-update'
const INSTALL_EVENT = 'cleany:install-update'

type Phase =
  | { kind: 'checking' }
  | { kind: 'upToDate'; version: string }
  | { kind: 'checkFailed' }
  | { kind: 'available'; info: UpdateInfo }
  | { kind: 'downloading'; info: UpdateInfo; received: number; total: number | null }
  | { kind: 'installing' }
  | { kind: 'failed' }

/**
 * Installs once: the button and the menu can both ask while a download runs,
 * and a second request would fail and show an error over the progress.
 */
function startInstall(info: UpdateInfo, setPhase: (phase: Phase) => void, busy: { current: boolean }) {
  if (busy.current) return
  busy.current = true
  setPhase({ kind: 'downloading', info, received: 0, total: null })
  installUpdate((event) =>
    setPhase(
      event.kind === 'installing'
        ? { kind: 'installing' }
        : { kind: 'downloading', info, received: event.received, total: event.total },
    ),
  ).catch(() => {
    busy.current = false
    setPhase({ kind: 'failed' })
  })
}

/**
 * A newer release, above every screen. Cleany checks once at launch and
 * installs only when the user asks; "Later" hides the bar until next launch.
 * The update item in the menus checks again, or installs what was found.
 */
export function UpdateStatus() {
  const { t, locale } = useT()
  const [phase, setPhase] = useState<Phase | null>(null)
  // The release the last check found, kept after "Later" for the menu item.
  const found = useRef<UpdateInfo | null>(null)
  // True from Install until the app restarts or the install fails.
  const installing = useRef(false)

  useEffect(() => {
    let live = true
    // Offline, or no release yet: say nothing.
    checkUpdate()
      .then((info) => {
        found.current = info
        if (live && info && !installing.current) setPhase({ kind: 'available', info })
      })
      .catch(() => undefined)

    // Asked from the menu, so the result is always shown.
    const check = () => {
      if (installing.current) return
      setPhase({ kind: 'checking' })
      Promise.all([checkUpdate(), getAppInfo()])
        .then(([info, app]) => {
          found.current = info
          if (live && !installing.current)
            setPhase(info ? { kind: 'available', info } : { kind: 'upToDate', version: app.version })
        })
        .catch(() => live && setPhase({ kind: 'checkFailed' }))
    }
    const install = () => {
      const info = found.current
      if (!info || installing.current) return
      if (info.canInstall) startInstall(info, setPhase, installing)
      else setPhase({ kind: 'available', info })
    }
    window.addEventListener(CHECK_EVENT, check)
    window.addEventListener(INSTALL_EVENT, install)
    return () => {
      live = false
      window.removeEventListener(CHECK_EVENT, check)
      window.removeEventListener(INSTALL_EVENT, install)
    }
  }, [])

  if (!phase) return null
  const frame = 'mx-4 mt-4 flex shrink-0 items-center gap-3 rounded-card bg-raised py-2 pr-2 pl-4 enter-fade max-sm:mx-3'
  const dismiss = () => setPhase(null)

  if (phase.kind === 'checking') {
    return (
      <div role="status" className={frame}>
        <p className="min-w-0 flex-1 py-1.5 text-secondary text-ink">{t('update.checking')}</p>
      </div>
    )
  }

  if (phase.kind === 'upToDate' || phase.kind === 'checkFailed') {
    return (
      <div role="status" className={frame}>
        <p className="min-w-0 flex-1 text-secondary text-ink">
          {phase.kind === 'upToDate' ? t('update.upToDate', { version: phase.version }) : t('update.checkFailed')}
        </p>
        <Button onClick={dismiss}>{t('update.close')}</Button>
      </div>
    )
  }

  if (phase.kind === 'available') {
    const { info } = phase
    if (!info.canInstall) {
      return (
        <div role="status" className={frame}>
          <p className="min-w-0 flex-1 text-secondary text-ink">{t('update.manual', { version: info.version })}</p>
          <Button onClick={dismiss}>{t('update.close')}</Button>
        </div>
      )
    }
    const install = () => startInstall(info, setPhase, installing)
    return (
      <div role="status" className={frame}>
        <p className="min-w-0 flex-1 text-secondary text-ink">{t('update.ready', { version: info.version })}</p>
        <Button onClick={dismiss}>{t('update.later')}</Button>
        <Button variant="primary" onClick={install}>
          {t('update.install')}
        </Button>
      </div>
    )
  }

  if (phase.kind === 'downloading') {
    const { info, received, total } = phase
    const ratio = total ? Math.min(1, received / total) : null
    return (
      <div role="status" className={frame}>
        <div className="flex min-w-0 flex-1 flex-col gap-1.5 py-1">
          <p className="truncate text-secondary text-ink">{t('update.downloading', { version: info.version })}</p>
          <div className="flex items-center gap-3">
            {ratio !== null && (
              <span aria-hidden className="block h-1 w-40 shrink-0 overflow-hidden rounded-full bg-card max-sm:w-24">
                <span
                  className="block h-full origin-left rounded-full bg-accent transition-transform duration-150 ease-out motion-reduce:transition-none"
                  style={{ transform: `scaleX(${ratio})` }}
                />
              </span>
            )}
            <span className="text-secondary text-ink-2 tabular-nums">
              {formatBytes(received, locale)}
              {total !== null && ` / ${formatBytes(total, locale)}`}
              {ratio !== null && ` · ${formatShare(ratio, 1, locale)}`}
            </span>
          </div>
        </div>
      </div>
    )
  }

  if (phase.kind === 'installing') {
    return (
      <div role="status" className={frame}>
        <p className="min-w-0 flex-1 py-1.5 text-secondary text-ink">{t('update.installing')}</p>
      </div>
    )
  }

  return (
    <div role="alert" className={frame}>
      <p className="min-w-0 flex-1 text-secondary text-ink-2">{t('update.failed')}</p>
      <Button onClick={dismiss}>{t('update.close')}</Button>
    </div>
  )
}
