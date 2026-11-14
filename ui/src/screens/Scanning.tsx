import { useEffect } from 'react'
import { Button } from '../components/ui'
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

  // Values update in place, without animation (DESIGN_MAP → Motion).
  return (
    <section aria-labelledby="scan-title" className="mx-auto w-full max-w-200 px-8 py-10">
      <h1 id="scan-title" className="truncate text-title font-medium" title={target.name}>
        {t('scan.title', { name: target.name })}
      </h1>
      <p className="mt-1 truncate font-mono text-meta text-ink-3" title={target.path}>
        {target.path}
      </p>

      <div className="mt-8 rounded-panel border border-hairline bg-card p-6">
        <p className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
          <span className="text-figure font-medium tabular-nums">{progress ? bytes(found) : '—'}</span>
          <span className="text-ink-2">
            {used === undefined ? t('scan.found') : t('scan.foundOf', { used: bytes(used) })}
          </span>
        </p>
        {used !== undefined && (
          <span
            role="img"
            aria-label={t('scan.progressLabel', { found: bytes(found), used: bytes(used) })}
            className="mt-5 block h-1.5 w-full overflow-hidden rounded-full border border-hairline bg-raised"
          >
            <span
              className="block h-full origin-left bg-ink-2"
              style={{ transform: `scaleX(${ratio})` }}
            />
          </span>
        )}
        <p className="mt-4 text-secondary text-ink-2 tabular-nums">
          {progress
            ? t('scan.counts', {
                files: formatCount(progress.files, locale),
                dirs: formatCount(progress.dirs, locale),
                elapsed: formatDuration(progress.elapsedMs, locale),
              })
            : t('scan.starting')}
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
