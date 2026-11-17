import { useEffect, useId, useRef, useState } from 'react'
import type { Locale } from '../lib/i18n'
import { launchAtLogin, setLaunchAtLogin } from '../lib/ipc'
import { useStore } from '../store'
import { useT } from '../useT'
import { Button, GearIcon, IconButton } from './ui'

/** Each language names itself, so it is findable whatever the UI language. */
const LOCALES: { code: Locale; name: string }[] = [
  { code: 'tr', name: 'Türkçe' },
  { code: 'en', name: 'English' },
]

/**
 * The Settings button in the header and the dialog it opens: the interface
 * language, and whether items can be deleted without the trash.
 */
export function SettingsDialog() {
  const { t } = useT()
  const [open, setOpen] = useState(false)
  const button = useRef<HTMLButtonElement>(null)
  const wasOpen = useRef(false)

  // Focus goes back to the button only once the modal dialog is closed:
  // while it is open, everything outside it is inert.
  useEffect(() => {
    if (wasOpen.current && !open) button.current?.focus()
    wasOpen.current = open
  }, [open])

  return (
    <>
      <IconButton
        ref={button}
        label={t('settings.open')}
        icon={<GearIcon />}
        look="quiet"
        align="end"
        aria-haspopup="dialog"
        onClick={() => setOpen(true)}
      />
      {open && <Settings onClose={() => setOpen(false)} />}
    </>
  )
}

function Settings({ onClose }: { onClose: () => void }) {
  const { t, locale } = useT()
  const setLocale = useStore((s) => s.setLocale)
  const permanentDelete = useStore((s) => s.permanentDelete)
  const setPermanentDelete = useStore((s) => s.setPermanentDelete)
  const ref = useRef<HTMLDialogElement>(null)
  const titleId = useId()
  const permanentId = useId()
  const noteId = useId()
  const removalId = useId()
  const backgroundId = useId()
  const loginId = useId()
  const loginNoteId = useId()
  // `null` until the OS answers; the checkbox waits for it.
  const [login, setLogin] = useState<boolean | null>(null)
  const [loginFailed, setLoginFailed] = useState(false)

  useEffect(() => {
    let live = true
    launchAtLogin()
      .then((on) => live && setLogin(on))
      .catch(() => live && setLogin(false))
    return () => {
      live = false
    }
  }, [])

  const toggleLogin = (on: boolean) => {
    setLoginFailed(false)
    setLogin(on)
    setLaunchAtLogin(on)
      .then(setLogin)
      .catch(() => {
        setLogin(!on)
        setLoginFailed(true)
      })
  }

  useEffect(() => {
    const dialog = ref.current
    dialog?.showModal()
    return () => dialog?.close()
  }, [])

  return (
    <dialog
      ref={ref}
      aria-labelledby={titleId}
      onCancel={(e) => {
        e.preventDefault()
        onClose()
      }}
      className="m-auto w-[calc(100%-2rem)] max-w-104 rounded-panel border border-dialog-edge bg-card p-6 text-ink shadow-dialog backdrop:bg-deepest/40"
    >
      <h2 id={titleId} className="text-strong font-medium">
        {t('settings.title')}
      </h2>
      <fieldset className="mt-4">
        <legend className="text-secondary font-medium text-ink-2">{t('settings.language')}</legend>
        <div className="mt-2 flex flex-col gap-2">
          {LOCALES.map((l) => (
            <label key={l.code} lang={l.code} className="flex cursor-pointer items-center gap-3 text-body text-ink">
              <input
                type="radio"
                name="locale"
                value={l.code}
                checked={l.code === locale}
                onChange={() => setLocale(l.code)}
                className="size-4 shrink-0 cursor-pointer accent-accent"
              />
              {l.name}
            </label>
          ))}
        </div>
      </fieldset>
      <section aria-labelledby={removalId} className="mt-6 border-t border-hairline pt-4">
        <h3 id={removalId} className="text-secondary font-medium text-ink-2">
          {t('settings.removal')}
        </h3>
        <div className="mt-2 flex items-start gap-3">
          <input
            id={permanentId}
            type="checkbox"
            checked={permanentDelete}
            onChange={(e) => setPermanentDelete(e.target.checked)}
            aria-describedby={noteId}
            className="mt-0.5 size-4 shrink-0 cursor-pointer accent-accent"
          />
          <div className="flex flex-col gap-1">
            <label htmlFor={permanentId} className="cursor-pointer text-body text-ink">
              {t('settings.permanent')}
            </label>
            <p id={noteId} className="text-body text-ink-2">
              {t('settings.permanentNote')}
            </p>
          </div>
        </div>
      </section>
      <section aria-labelledby={backgroundId} className="mt-6 border-t border-hairline pt-4">
        <h3 id={backgroundId} className="text-secondary font-medium text-ink-2">
          {t('settings.background')}
        </h3>
        <div className="mt-2 flex items-start gap-3">
          <input
            id={loginId}
            type="checkbox"
            checked={login ?? false}
            disabled={login === null}
            onChange={(e) => toggleLogin(e.target.checked)}
            aria-describedby={loginNoteId}
            className="mt-0.5 size-4 shrink-0 cursor-pointer accent-accent disabled:cursor-default"
          />
          <div className="flex flex-col gap-1">
            <label htmlFor={loginId} className="cursor-pointer text-body text-ink">
              {t('settings.login')}
            </label>
            <p id={loginNoteId} className="text-body text-ink-2">
              {t('settings.loginNote')}
            </p>
            {loginFailed && (
              <p role="status" className="text-body text-ink">
                {t('settings.loginFailed')}
              </p>
            )}
          </div>
        </div>
      </section>
      <div className="mt-6 flex justify-end">
        <Button onClick={onClose}>{t('settings.close')}</Button>
      </div>
    </dialog>
  )
}
