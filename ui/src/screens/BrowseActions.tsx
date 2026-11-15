import { useEffect, useState } from 'react'
import { Button } from '../components/ui'
import { getAppInfo } from '../lib/ipc'
import { useT } from '../useT'
import type { ActionTarget, Actions } from './useBrowseActions'

type Os = 'macos' | 'windows' | 'other'
let osOnce: Promise<Os> | null = null
/** The OS, asked once per app run; it names the file manager. */
function useOs(): Os | null {
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

const REVEAL_TEXT = { macos: 'actions.revealMac', windows: 'actions.revealWin', other: 'actions.revealOther' } as const

const Kbd = ({ children }: { children: string }) => (
  <kbd aria-hidden className="font-mono text-meta font-normal text-ink-3">
    {children}
  </kbd>
)

/** The three actions under the view, with their feedback line. */
export function ActionBar({ target, actions }: { target: ActionTarget; actions: Actions }) {
  const { t } = useT()
  const os = useOs()
  const mac = os === 'macos'
  const { feedback } = actions
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span id="browse-action-target" className="sr-only">
        {t('actions.target', { name: target.name })}
      </span>
      <div role="group" aria-label={t('actions.label')} className="flex flex-wrap gap-2">
        <Button onClick={actions.reveal} aria-describedby="browse-action-target" aria-keyshortcuts="R">
          {t(REVEAL_TEXT[os ?? 'other'])}
          <Kbd>R</Kbd>
        </Button>
        <Button onClick={actions.open} aria-describedby="browse-action-target" aria-keyshortcuts="O">
          {t('actions.open')}
          <Kbd>O</Kbd>
        </Button>
        <Button
          onClick={actions.copy}
          aria-describedby="browse-action-target"
          aria-keyshortcuts={mac ? 'Meta+C' : 'Control+C'}
        >
          {t('actions.copy')}
          <Kbd>{mac ? '⌘C' : 'Ctrl+C'}</Kbd>
        </Button>
      </div>
      {/* Both regions stay mounted so a change is announced. Shown at once:
          the feedback answers a keypress, and keys don't animate. */}
      <p role="status" className="text-secondary text-ink-2">
        {feedback?.kind === 'copied' ? feedback.text : ''}
      </p>
      <p role="alert" className="text-secondary text-ink">
        {feedback?.kind === 'error' ? feedback.text : ''}
      </p>
    </div>
  )
}
