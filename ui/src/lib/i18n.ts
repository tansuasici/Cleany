// Typed TR/EN dictionary. `tr` is the source of keys; `en` must provide the
// same keys with the same parameter shapes, so a missing translation is a
// `tsc` error rather than a runtime surprise.

export type Locale = 'tr' | 'en'

type Count = { n: string; count: number }

const tr = {
  'app.name': 'Cleany',
  'app.switchLocale': 'English',
  'app.switchLocaleLabel': 'Dili İngilizce yap',
  'app.back': 'Birimler',

  'volumes.title': 'Yer nereye gitti?',
  'volumes.lede': 'Bir birim seç ve tara. Tarama yalnızca okur, hiçbir şeyi silmez.',
  'volumes.loading': 'Birimler okunuyor…',
  'volumes.empty': 'Bağlı bir birim bulunamadı.',
  'volumes.error': 'Birimler okunamadı.',
  'volumes.retry': 'Tekrar dene',
  'volumes.free': 'boş',
  'volumes.usage': (p: { used: string; total: string }) => `${p.used} / ${p.total} dolu`,
  'volumes.removable': 'Çıkarılabilir',
  'volumes.scan': 'Tara',
  'volumes.scanLabel': (p: { name: string }) => `${p.name} birimini tara`,
  'volumes.open': 'Dökümü aç',
  'volumes.openLabel': (p: { name: string }) => `${p.name} dökümünü aç`,
  'volumes.rescan': 'Yeniden tara',
  'volumes.rescanLabel': (p: { name: string }) => `${p.name} birimini yeniden tara`,
  'volumes.lastScan': 'Çubuk son taramadaki en büyük kalemlere göre bölündü.',

  'scan.title': (p: { name: string }) => `${p.name} taranıyor`,
  'scan.found': 'bulundu',
  'scan.foundOf': (p: { used: string }) => `bulundu, yaklaşık ${p.used} içinden`,
  'scan.progressLabel': (p: { found: string; used: string }) => `${p.used} içinden ${p.found} bulundu`,
  'scan.counts': (p: { files: string; dirs: string; elapsed: string }) =>
    `${p.files} dosya · ${p.dirs} klasör · ${p.elapsed}`,
  'scan.starting': 'Tarama başlıyor…',
  'scan.cancel': 'İptal et',
  'scan.cancelHint': 'Esc ile de iptal edebilirsin. Tarama hiçbir şeyi değiştirmez.',

  'cancelled.title': 'Tarama iptal edildi',
  'cancelled.body': 'Yarım kalan sonuç bırakıldı. İstersen yeniden başlatabilirsin.',
  'failed.title': 'Tarama tamamlanamadı',
  'retry': 'Tekrar tara',
  'backToVolumes': 'Birimlere dön',

  'summary.title': (p: { name: string }) => `${p.name} dökümü`,
  'summary.lede': (p: { counted: string }) => `Bu taramada ${p.counted} kullanılan alan sayıldı.`,
  'summary.ledeFree': (p: { counted: string; free: string }) =>
    `Bu taramada ${p.counted} kullanılan alan sayıldı; birimde ${p.free} boş.`,
  'summary.items': 'En büyük kalemler',
  'summary.share': 'Pay',
  'summary.childrenError': 'İçerik okunamadı.',
  'summary.loadingItems': 'Döküm hazırlanıyor…',
  'summary.itemsError': 'Döküm okunamadı.',
  'summary.noItems': 'Bu birimde sayılacak bir şey yok.',
  'summary.others': (p: Count) => `Diğer ${p.n} öğe`,
  'summary.sum': 'Toplam',
  'summary.notCounted': 'Sayılmayanlar',
  'summary.hardlinks': (p: Count) =>
    `${p.n} sabit bağlantı (hardlink) bir kez sayıldı; kopyaları toplama eklenmedi.`,
  'summary.placeholders': (p: Count) =>
    `${p.n} iCloud/OneDrive yer tutucusu diskte yer kaplamıyor; indirilmedi, içine girilmedi.`,
  'summary.skippedMounts': (p: Count) =>
    `${p.n} başka birim (bağlama noktası) bu taramanın dışında bırakıldı.`,
  'summary.unreadable': (p: Count) => `${p.n} öğe okunamadı.`,
  'summary.unreadableShow': (p: Count) => `İlk ${p.n} tanesini göster`,
  'summary.stats': (p: { files: string; dirs: string; elapsed: string }) =>
    `${p.files} dosya ve ${p.dirs} klasör ${p.elapsed} içinde tarandı.`,
  'summary.browseAll': 'Tümüne göz at',
  'summary.expandLabel': (p: { name: string }) => `${p.name} içeriğini burada göster`,

  'browse.back': 'Döküm',
  'browse.location': 'Konum',
  'browse.tableLabel': (p: { name: string }) => `${p.name} içeriği`,
  'browse.name': 'Ad',
  'browse.size': 'Boyut',
  'browse.share': 'Pay',
  'browse.files': 'Dosya',
  'browse.modified': 'Değiştirilme',
  'browse.count': (p: Count) => `${p.n} öğe`,
  'browse.loading': 'İçerik okunuyor…',
  'browse.empty': 'Bu klasör boş.',
  'browse.hint': '↑ ↓ ile seç, Enter ile içine gir, ← ile üst klasöre çık, Esc ile döküme dön.',

  'duration.seconds': (p: { s: string }) => `${p.s} sn`,
  'duration.minutes': (p: { m: string; s: string }) => `${p.m} dk ${p.s} sn`,
}

type Dict = {
  [K in keyof typeof tr]: (typeof tr)[K] extends string ? string : (typeof tr)[K]
}

const plural = (one: string, many: string) => (count: number) => (count === 1 ? one : many)
const items = plural('item', 'items')
const hardlinks = plural('hard link was', 'hard links were')
const placeholders = plural('placeholder takes', 'placeholders take')
const volumes = plural('other volume (mount point) was', 'other volumes (mount points) were')
const entries = plural('entry', 'entries')

const en: Dict = {
  'app.name': 'Cleany',
  'app.switchLocale': 'Türkçe',
  'app.switchLocaleLabel': 'Switch language to Turkish',
  'app.back': 'Volumes',

  'volumes.title': 'Where did the space go?',
  'volumes.lede': 'Pick a volume and scan it. Scanning only reads; it never deletes anything.',
  'volumes.loading': 'Reading volumes…',
  'volumes.empty': 'No mounted volumes found.',
  'volumes.error': 'Could not read the volumes.',
  'volumes.retry': 'Try again',
  'volumes.free': 'free',
  'volumes.usage': (p) => `${p.used} of ${p.total} used`,
  'volumes.removable': 'Removable',
  'volumes.scan': 'Scan',
  'volumes.scanLabel': (p) => `Scan ${p.name}`,
  'volumes.open': 'Open breakdown',
  'volumes.openLabel': (p) => `Open the ${p.name} breakdown`,
  'volumes.rescan': 'Scan again',
  'volumes.rescanLabel': (p) => `Scan ${p.name} again`,
  'volumes.lastScan': 'The bar is split by the largest items of the last scan.',

  'scan.title': (p) => `Scanning ${p.name}`,
  'scan.found': 'found',
  'scan.foundOf': (p) => `found, of roughly ${p.used}`,
  'scan.progressLabel': (p) => `${p.found} of ${p.used} found`,
  'scan.counts': (p) => `${p.files} files · ${p.dirs} folders · ${p.elapsed}`,
  'scan.starting': 'Starting the scan…',
  'scan.cancel': 'Cancel',
  'scan.cancelHint': 'Esc cancels too. Scanning changes nothing.',

  'cancelled.title': 'Scan cancelled',
  'cancelled.body': 'The partial result was dropped. You can start again.',
  'failed.title': 'The scan could not finish',
  'retry': 'Scan again',
  'backToVolumes': 'Back to volumes',

  'summary.title': (p) => `${p.name} breakdown`,
  'summary.lede': (p) => `This scan counted ${p.counted} of used space.`,
  'summary.ledeFree': (p) => `This scan counted ${p.counted} of used space; ${p.free} is free on the volume.`,
  'summary.items': 'Largest items',
  'summary.share': 'Share',
  'summary.childrenError': 'Could not read the contents.',
  'summary.loadingItems': 'Preparing the breakdown…',
  'summary.itemsError': 'Could not read the breakdown.',
  'summary.noItems': 'Nothing to count on this volume.',
  'summary.others': (p) => `${p.n} other ${items(p.count)}`,
  'summary.sum': 'Total',
  'summary.notCounted': 'Not counted',
  'summary.hardlinks': (p) =>
    `${p.n} ${hardlinks(p.count)} counted once; the copies add nothing to the total.`,
  'summary.placeholders': (p) =>
    `${p.n} iCloud/OneDrive ${placeholders(p.count)} no disk space; not downloaded or opened.`,
  'summary.skippedMounts': (p) => `${p.n} ${volumes(p.count)} left out of this scan.`,
  'summary.unreadable': (p) => `${p.n} ${entries(p.count)} could not be read.`,
  'summary.unreadableShow': (p) => `Show the first ${p.n}`,
  'summary.stats': (p) => `${p.files} files and ${p.dirs} folders scanned in ${p.elapsed}.`,
  'summary.browseAll': 'Browse all',
  'summary.expandLabel': (p) => `Show what is in ${p.name} here`,

  'browse.back': 'Breakdown',
  'browse.location': 'Location',
  'browse.tableLabel': (p) => `Contents of ${p.name}`,
  'browse.name': 'Name',
  'browse.size': 'Size',
  'browse.share': 'Share',
  'browse.files': 'Files',
  'browse.modified': 'Modified',
  'browse.count': (p) => `${p.n} ${items(p.count)}`,
  'browse.loading': 'Reading the contents…',
  'browse.empty': 'This folder is empty.',
  'browse.hint': '↑ ↓ to select, Enter to open, ← to go up, Esc for the breakdown.',

  'duration.seconds': (p) => `${p.s} s`,
  'duration.minutes': (p) => `${p.m} min ${p.s} s`,
}

const dictionaries: Record<Locale, Dict> = { tr, en }

export type MessageKey = keyof Dict
export type Params<K extends MessageKey> = Dict[K] extends (p: infer P) => string ? [P] : []

/** Translates `key` for `locale`; keys with parameters require them. */
export function translate<K extends MessageKey>(
  locale: Locale,
  key: K,
  ...params: Params<K>
): string {
  const message: string | ((p: never) => string) = dictionaries[locale][key]
  return typeof message === 'string' ? message : message(params[0] as never)
}

export function detectLocale(): Locale {
  return navigator.language.toLowerCase().startsWith('tr') ? 'tr' : 'en'
}
