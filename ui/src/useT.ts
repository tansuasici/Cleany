import { useCallback } from 'react'
import { type MessageKey, type Params, translate } from './lib/i18n'
import { useStore } from './store'

type Translate = <K extends MessageKey>(key: K, ...params: Params<K>) => string

/** The current locale and its translator; re-renders when the locale changes. */
export function useT() {
  const locale = useStore((s) => s.locale)
  const t = useCallback<Translate>((key, ...params) => translate(locale, key, ...params), [locale])
  return { locale, t }
}
