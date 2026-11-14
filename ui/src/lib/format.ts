import { type Locale, translate } from './i18n'

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'] as const

/**
 * Formats a byte count with decimal units (1 GB = 10^9 bytes, as Finder and
 * GNOME show), one fraction digit below 100 and none above: `48,2 GB`.
 */
export function formatBytes(bytes: number, locale: Locale): string {
  let value = Math.max(0, bytes)
  let unit = 0
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000
    unit += 1
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1
  const number = new Intl.NumberFormat(locale, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value)
  return `${number} ${UNITS[unit]}`
}

/** Formats a count with the locale's grouping: `1.234.567`. */
export function formatCount(count: number, locale: Locale): string {
  return new Intl.NumberFormat(locale).format(count)
}

/** Formats elapsed milliseconds: `12,3 sn`, or `4 dk 05 sn` past a minute. */
export function formatDuration(ms: number, locale: Locale): string {
  const seconds = Math.max(0, ms) / 1000
  if (seconds < 60) {
    const tenths = new Intl.NumberFormat(locale, {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    })
    // A small volume finishes in milliseconds; "0,0 sn" reads as "didn't run".
    const s = seconds < 0.1 ? `< ${tenths.format(0.1)}` : tenths.format(seconds)
    return translate(locale, 'duration.seconds', { s })
  }
  const whole = Math.floor(seconds)
  const m = formatCount(Math.floor(whole / 60), locale)
  const s = String(whole % 60).padStart(2, '0')
  return translate(locale, 'duration.minutes', { m, s })
}

/** Formats `part` as a whole-percent share of `whole`: `%38`, `< %1`. */
export function formatShare(part: number, whole: number, locale: Locale): string {
  if (whole <= 0) return '—'
  const ratio = Math.max(0, part) / whole
  const percent = new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 0 })
  return ratio > 0 && ratio < 0.005 ? `< ${percent.format(0.01)}` : percent.format(ratio)
}

/** Formats seconds since the Unix epoch as a medium date; unknown is `—`. */
export function formatDate(seconds: number, locale: Locale): string {
  if (seconds <= 0) return '—'
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(seconds * 1000)
}

/**
 * Shortens `name` to at most `max` characters by cutting its middle, so both
 * the start and the extension stay readable: `rapor-2026-…-son.pdf`.
 */
export function truncateMiddle(name: string, max: number): string {
  const chars = Array.from(name)
  if (chars.length <= max) return name
  const keep = max - 1
  const head = Math.ceil(keep / 2)
  return `${chars.slice(0, head).join('')}…${chars.slice(chars.length - (keep - head)).join('')}`
}

/** How long ago `at` (ms since the epoch) was, from `now`: `5 dk önce`. */
export function formatAgo(at: number, now: number, locale: Locale): string {
  const seconds = Math.max(0, Math.round((now - at) / 1000))
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto', style: 'short' })
  if (seconds < 60) return translate(locale, 'ago.now')
  const minutes = Math.round(seconds / 60)
  if (minutes < 60) return rtf.format(-minutes, 'minute')
  const hours = Math.round(minutes / 60)
  if (hours < 24) return rtf.format(-hours, 'hour')
  return rtf.format(-Math.round(hours / 24), 'day')
}
