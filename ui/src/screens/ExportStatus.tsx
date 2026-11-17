import { Button } from '../components/ui'
import { useExport } from '../exportJob'
import { formatBytes, formatCount, formatShare } from '../lib/format'
import { revealExport } from '../lib/ipc'
import { useOs } from '../useOs'
import { useT } from '../useT'

/**
 * The export on show, above every screen: the file and the rows written while
 * it runs, with a stop button, then what was written and where.
 */
export function ExportStatus() {
  const { t, locale } = useT()
  const job = useExport((s) => s.job)
  const stop = useExport((s) => s.stop)
  const dismiss = useExport((s) => s.dismiss)
  const os = useOs()
  if (!job) return null

  const frame = 'mx-4 mt-4 flex shrink-0 items-center gap-3 rounded-card bg-raised py-2 pr-2 pl-4 enter-fade max-sm:mx-3'
  const close = <Button onClick={dismiss}>{t('export.close')}</Button>

  if (job.phase === 'running') {
    // The total is an upper bound, so the share stays under 100 % until the end.
    const ratio = job.total ? Math.min(0.99, job.rows / job.total) : null
    return (
      <div role="status" className={frame}>
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <p className="truncate text-secondary text-ink">{t('export.running', { file: job.fileName })}</p>
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
              {t('export.rows', { n: formatCount(job.rows, locale), count: job.rows })}
              {ratio !== null && ` · ${formatShare(ratio, 1, locale)}`}
            </span>
          </div>
        </div>
        <Button onClick={stop} disabled={job.stopping}>
          {t(job.stopping ? 'export.stopping' : 'export.stop')}
        </Button>
      </div>
    )
  }

  if (job.phase === 'done') {
    const { rows, bytes, fileName } = job.report
    const reveal = os === 'windows' ? 'actions.revealWin' : os === 'macos' ? 'actions.revealMac' : 'actions.revealOther'
    return (
      <div role="status" className={frame}>
        <p className="min-w-0 flex-1 text-secondary text-ink">
          {t('export.done', {
            n: formatCount(rows, locale),
            count: rows,
            file: fileName,
            size: formatBytes(bytes, locale),
          })}
        </p>
        <Button onClick={() => void revealExport().catch(() => undefined)}>{t(reveal)}</Button>
        {close}
      </div>
    )
  }

  return (
    <div role={job.phase === 'failed' ? 'alert' : 'status'} className={frame}>
      <p className="min-w-0 flex-1 text-secondary text-ink-2">
        {t(job.phase === 'stopped' ? 'export.stopped' : job.stale ? 'export.stale' : 'export.error')}
      </p>
      {close}
    </div>
  )
}
