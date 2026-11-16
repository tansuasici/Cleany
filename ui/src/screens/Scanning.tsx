import { useEffect, useRef, useState } from 'react'
import { Button, NAME, Named, Size } from '../components/ui'
import { formatBytes, formatCount, formatDuration } from '../lib/format'
import { type Screen, useStore } from '../store'
import { useT } from '../useT'

type Props = { screen: Extract<Screen, { kind: 'scanning' }> }

/**
 * One figure while the scan runs: bytes found so far, against the volume's
 * used space when it is known. The bar stops short of full, because the
 * volume's own count and the scan's never match exactly.
 */
export function ScanningScreen({ screen }: Props) {
  const { t, locale } = useT()
  const cancel = useStore((s) => s.cancel)
  const { target, progress } = screen
  const bytes = (n: number) => formatBytes(n, locale)

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') void cancel()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [cancel])

  const found = progress?.allocated ?? 0
  const used = target.used !== undefined && target.used > 0 ? target.used : undefined
  const ratio = used === undefined ? 0 : Math.min(0.98, found / used)
  const announced = useAnnounced(progress ? bytes(found) : null)

  // Values update in place, without animation (DESIGN_MAP → Motion).
  return (
    <section aria-labelledby="scan-title" className="mx-auto w-full max-w-200 px-4 py-10 sm:px-8 sm:py-16">
      <h1 id="scan-title" className="text-title font-medium">
        <Named text={t('scan.title', { name: NAME })} name={target.name} />
      </h1>

      <div className="mt-8 rounded-panel border border-hairline bg-card p-4 sm:p-6">
        {progress === null ? (
          // Same height as the figure line, so nothing jumps when the first count lands.
          <p className="flex min-h-11 items-center text-ink-2">{t('scan.starting')}</p>
        ) : (
          <>
            <p className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
              <Size value={bytes(found)} className="text-display font-medium" />
              {used !== undefined && <span className="text-ink-2">{t('scan.foundOf', { used: bytes(used) })}</span>}
            </p>
            {used !== undefined && (
              <span
                role="img"
                aria-label={t('scan.progressLabel', { found: bytes(found), used: bytes(used) })}
                className="mt-5 block h-2 w-full overflow-hidden rounded-tag bg-raised"
              >
                <span className="block h-full origin-left bg-ink-2" style={{ transform: `scaleX(${ratio})` }} />
              </span>
            )}
            <dl className="mt-6 flex max-w-100 flex-col gap-1 text-secondary text-ink-2">
              <Counter label={t('scan.files')} value={formatCount(progress.files, locale)} />
              <Counter label={t('scan.dirs')} value={formatCount(progress.dirs, locale)} />
              <Counter label={t('scan.elapsed')} value={formatDuration(progress.elapsedMs, locale)} />
            </dl>
          </>
        )}
        {/* The figures change ten times a second; a screen reader hears one now and then. */}
        <p role="status" className="sr-only">
          {announced && t('scan.status', { found: announced })}
        </p>
      </div>

      <div className="mt-6 flex items-center gap-4">
        <Button onClick={() => void cancel()} autoFocus>
          {t('scan.cancel')}
        </Button>
        <p className="text-secondary text-ink-2">{t('scan.cancelHint')}</p>
      </div>
    </section>
  )
}

/** A live counter as a leader line: `Dosya ······ 12.408`. */
function Counter({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-baseline gap-2">
      <dt className="shrink-0">{label}</dt>
      <span className="leader" aria-hidden />
      <dd className="shrink-0 font-mono text-ink tabular-nums">{value}</dd>
    </div>
  )
}

/** How often the scan's status line is refreshed for screen readers. */
const ANNOUNCE_MS = 10_000

/** `value`, but changing at most once every `ANNOUNCE_MS`. */
function useAnnounced(value: string | null): string | null {
  const latest = useRef(value)
  useEffect(() => {
    latest.current = value
  })
  const [announced, setAnnounced] = useState<string | null>(null)
  useEffect(() => {
    const id = window.setInterval(() => setAnnounced(latest.current), ANNOUNCE_MS)
    return () => window.clearInterval(id)
  }, [])
  return announced
}
