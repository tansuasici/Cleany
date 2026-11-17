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
    <section aria-labelledby="ended-title" className="mx-auto w-full max-w-200 px-8 py-16">
      <h1 id="ended-title" className="text-title font-medium">
        {t(failed ? 'failed.title' : 'cancelled.title')}
      </h1>
      {failed ? (
        <div role="alert" className="mt-6 flex flex-col gap-3">
          <p className="text-ink-2">{t('failed.lead')}</p>
          <details>
            <summary className="flex min-h-6 w-fit cursor-pointer items-center rounded-tag text-secondary text-ink-2 hover:text-ink">
              {t('failed.detail')}
            </summary>
            <p className="mt-2 break-words rounded-control bg-raised p-3 font-mono text-secondary text-ink">{screen.message}</p>
          </details>
        </div>
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
