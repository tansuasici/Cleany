import { type KeyboardEvent, useEffect, useId, useRef, useState } from 'react'
import type { Locale } from '../lib/i18n'
import { useStore } from '../store'
import { useT } from '../useT'
import { CheckIcon, ChevronIcon } from './ui'

/** Each language names itself, so it is findable whatever the UI language. */
const LOCALES: { code: Locale; name: string }[] = [
  { code: 'tr', name: 'Türkçe' },
  { code: 'en', name: 'English' },
]

/**
 * The language menu in the header: a menu button with one radio item per
 * language. Arrow keys move, Enter/Space picks, Escape and Tab close.
 */
export function LocaleMenu() {
  const { t, locale } = useT()
  const setLocale = useStore((s) => s.setLocale)
  const [open, setOpen] = useState(false)
  const button = useRef<HTMLButtonElement>(null)
  const menu = useRef<HTMLDivElement>(null)
  const menuId = useId()
  const current = LOCALES.find((l) => l.code === locale) ?? LOCALES[0]

  const items = () => Array.from(menu.current?.querySelectorAll<HTMLElement>('[role=menuitemradio]') ?? [])
  const close = (refocus: boolean) => {
    setOpen(false)
    if (refocus) button.current?.focus()
  }

  // Opening moves focus to the current language.
  useEffect(() => {
    if (!open) return
    items()
      .find((el) => el.getAttribute('aria-checked') === 'true')
      ?.focus()
    const onPointer = (e: PointerEvent) => {
      const target = e.target as Node
      if (!menu.current?.contains(target) && !button.current?.contains(target)) setOpen(false)
    }
    document.addEventListener('pointerdown', onPointer)
    return () => document.removeEventListener('pointerdown', onPointer)
  }, [open])

  const pick = (code: Locale) => {
    setLocale(code)
    close(true)
  }

  const onButtonKey = (e: KeyboardEvent) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      setOpen(true)
    }
  }

  const onMenuKey = (e: KeyboardEvent) => {
    const list = items()
    const at = list.indexOf(document.activeElement as HTMLElement)
    const move = (i: number) => list[(i + list.length) % list.length]?.focus()
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        move(at + 1)
        break
      case 'ArrowUp':
        e.preventDefault()
        move(at - 1)
        break
      case 'Home':
        e.preventDefault()
        move(0)
        break
      case 'End':
        e.preventDefault()
        move(list.length - 1)
        break
      case 'Escape':
        e.preventDefault()
        close(true)
        break
      case 'Tab':
        close(false)
        break
    }
  }

  return (
    <div className="relative">
      <button
        ref={button}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-label={t('app.language', { name: current.name })}
        onClick={() => setOpen((o) => !o)}
        onKeyDown={onButtonKey}
        className="inline-flex h-8 items-center gap-1 rounded-control px-2 text-secondary font-medium text-ink-2 transition-[background-color,color] duration-150 ease-out hover:bg-raised hover:text-ink motion-reduce:transition-none"
      >
        <span aria-hidden>{current.code.toUpperCase()}</span>
        <span
          aria-hidden
          className={`inline-flex rotate-90 transition-transform duration-150 ease-out motion-reduce:transition-none ${open ? '-scale-x-100' : ''}`}
        >
          <ChevronIcon />
        </span>
      </button>
      {open && (
        <div
          ref={menu}
          id={menuId}
          role="menu"
          aria-label={t('app.languages')}
          onKeyDown={onMenuKey}
          className="absolute top-full right-0 z-10 mt-1 min-w-40 origin-top-right rounded-card border border-hairline bg-card p-1 transition-[opacity,transform] duration-150 ease-out starting:scale-[0.97] starting:opacity-0 motion-reduce:starting:scale-100"
        >
          {LOCALES.map((l) => {
            const checked = l.code === locale
            return (
              <button
                key={l.code}
                type="button"
                role="menuitemradio"
                aria-checked={checked}
                lang={l.code}
                tabIndex={-1}
                onClick={() => pick(l.code)}
                className="flex h-8 w-full items-center gap-2 rounded-tag px-2 text-left text-body text-ink outline-none hover:bg-raised focus:bg-raised"
              >
                <span className={`inline-flex text-accent ${checked ? '' : 'invisible'}`}>
                  <CheckIcon />
                </span>
                {l.name}
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
