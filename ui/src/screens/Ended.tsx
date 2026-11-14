import { Button } from '../components/ui'
import { type Screen, useStore } from '../store'
import { useT } from '../useT'

type Props = { screen: Extract<Screen, { kind: 'cancelled' } | { kind: 'failed' }> }

/** A scan that stopped without a result: cancelled by the user, or failed. */
export function EndedScreen({ screen }: Props) {
  const { t } = useT()
  const scan = useStore((s) => s.scan)
  const showVolumes = useStore((s) => s.showVolumes)
  const failed = screen.kind === 'failed'

  return (
    <section aria-labelledby="ended-title" className="mx-auto w-full max-w-200 px-8 py-10">
      <h1 id="ended-title" className="text-title font-medium">
        {t(failed ? 'failed.title' : 'cancelled.title')}
      </h1>
      <p className="mt-1 truncate font-mono text-meta text-ink-3" title={screen.target.path}>
        {screen.target.path}
      </p>
      {failed ? (
        <p role="alert" className="mt-6 break-words rounded-control border-l-2 border-negative bg-card p-3 font-mono text-secondary text-ink">
          {screen.message}
        </p>
      ) : (
        <p className="mt-6 text-ink-2">{t('cancelled.body')}</p>
      )}
      <div className="mt-6 flex gap-3">
        <Button variant="primary" onClick={() => void scan(screen.target)} autoFocus>
          {t('retry')}
        </Button>
        <Button onClick={showVolumes}>{t('backToVolumes')}</Button>
      </div>
    </section>
  )
}
