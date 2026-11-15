// Typed TR/EN dictionary. `tr` is the source of keys; `en` must provide the
// same keys with the same parameter shapes, so a missing translation is a
// `tsc` error rather than a runtime surprise.

export type Locale = 'tr' | 'en'

type Count = { n: string; count: number }

const tr = {
  'app.name': 'Cleany',
  'app.home': 'Cleany, birimlere dön',
  'app.language': (p: { name: string }) => `Dil: ${p.name}`,
  'app.languages': 'Dil',
  'app.volumes': 'Birimler',
  'app.trail': 'Konum',

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
  'volumes.scanLabel': (p: { name: string }) => `Tara: ${p.name}`,
  'volumes.open': 'Dökümü aç',
  'volumes.openLabel': (p: { name: string }) => `Dökümü aç: ${p.name}`,
  'volumes.rescan': 'Yeniden tara',
  'volumes.rescanLabel': (p: { name: string }) => `Yeniden tara: ${p.name}`,
  'volumes.lastScan': (p: { ago: string }) => `Son tarama: ${p.ago}. Çubuk en büyük kalemlere göre bölündü.`,
  'access.title': 'Tam Disk Erişimi verilmedi',
  'access.body':
    'macOS bazı klasörleri (Mail, Safari, başka uygulamaların verileri) Cleany’ye kapalı tutuyor. Tarama yine çalışır, ama bu klasörler toplama girmez.',
  'access.steps': 'Sistem Ayarları’nda Cleany’yi Tam Disk Erişimi listesinde açık konuma getir. macOS uygulamayı yeniden açmanı isteyebilir.',
  'access.open': 'Sistem Ayarları’nı aç',
  'access.dismiss': 'Şimdilik geç',
  'access.openError': 'Sistem Ayarları açılamadı. Gizlilik ve Güvenlik → Tam Disk Erişimi bölmesine elle gidebilirsin.',

  'scan.title': (p: { name: string }) => `${p.name} taranıyor`,
  'scan.foundOf': (p: { used: string }) => `yaklaşık ${p.used} içinden`,
  'scan.progressLabel': (p: { found: string; used: string }) => `${p.used} içinden ${p.found} bulundu`,
  'scan.files': 'Dosya',
  'scan.dirs': 'Klasör',
  'scan.elapsed': 'Süre',
  'scan.status': (p: { found: string }) => `${p.found} bulundu`,
  'scan.starting': 'Tarama başlıyor…',
  'scan.cancel': 'İptal et',
  'scan.cancelHint': 'Esc ile de iptal edebilirsin. Tarama hiçbir şeyi değiştirmez.',

  'cancelled.title': 'Tarama iptal edildi',
  'cancelled.body': 'Yarım kalan sonuç bırakıldı. İstersen yeniden başlatabilirsin.',
  'failed.title': 'Tarama tamamlanamadı',
  'failed.lead': 'Bir hata taramayı yarıda kesti; hiçbir şey değiştirilmedi. Sistemin verdiği ayrıntı:',
  'retry': 'Yeniden tara',
  'backToVolumes': 'Birimlere dön',

  'summary.title': (p: { name: string }) => `${p.name} dökümü`,
  'summary.free': (p: { free: string }) => `Birimde ${p.free} boş.`,
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
  'summary.noAccess': 'Tam Disk Erişimi olmadan korumalı klasörler okunamıyor.',
  'summary.stats': (p: { files: string; dirs: string; elapsed: string }) =>
    `${p.files} dosya ve ${p.dirs} klasör ${p.elapsed} içinde tarandı.`,
  'summary.browseAll': 'Tümüne göz at',
  'summary.open': 'Aç',
  'summary.openLabel': (p: { name: string }) => `Aç: ${p.name}`,

  'browse.all': 'Tümü',
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
  'browse.view': 'Görünüm',
  'browse.table': 'Tablo',
  'browse.map': 'Harita',
  'browse.mapLabel': (p: { name: string }) => `${p.name} haritası`,
  'browse.others': (p: Count) => `Diğer ${p.n} öğe`,
  'browse.fileCount': (p: Count) => `${p.n} dosya`,
  'browse.mapEmpty': 'Bu klasörde haritada gösterilecek bir boyut yok.',
  'browse.mapHint': 'Oklarla blok seç, Enter ya da tıklamayla içine gir, Backspace ile üst klasöre çık, Esc ile döküme dön.',
  'browse.rings': 'Halka',
  'browse.ringsLabel': (p: { name: string }) => `${p.name} halkaları`,
  'browse.ringsEmpty': 'Bu klasörde halkada gösterilecek bir boyut yok.',
  'browse.ringsHint': '← → ile dilim seç, Enter ya da tıklamayla içine gir, ortaya tıkla ya da Backspace ile üst klasöre çık, Esc ile döküme dön.',

  'actions.label': 'Seçili öğe',
  'actions.target': (p: { name: string }) => `Seçili: ${p.name}`,
  'actions.revealMac': "Finder'da göster",
  'actions.revealWin': "Gezgin'de göster",
  'actions.revealOther': 'Dosya yöneticisinde göster',
  'actions.open': 'Aç',
  'actions.copy': 'Yolu kopyala',
  'actions.copied': 'Yol kopyalandı.',
  'actions.gone': 'Bu öğe artık yok. Taramadan sonra silinmiş ya da taşınmış olabilir.',
  'actions.revealError': 'Dosya yöneticisi açılamadı.',
  'actions.openError': 'Bu öğe açılamadı.',
  'actions.copyError': 'Yol kopyalanamadı.',

  'ago.now': 'az önce',
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
  'app.home': 'Cleany, back to volumes',
  'app.language': (p) => `Language: ${p.name}`,
  'app.languages': 'Language',
  'app.volumes': 'Volumes',
  'app.trail': 'Location',

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
  'volumes.scanLabel': (p) => `Scan: ${p.name}`,
  'volumes.open': 'Open breakdown',
  'volumes.openLabel': (p) => `Open breakdown: ${p.name}`,
  'volumes.rescan': 'Scan again',
  'volumes.rescanLabel': (p) => `Scan again: ${p.name}`,
  'volumes.lastScan': (p) => `Last scan: ${p.ago}. The bar is split by the largest items.`,
  'access.title': 'Full Disk Access is off',
  'access.body':
    'macOS keeps some folders (Mail, Safari, other apps’ data) closed to Cleany. Scans still run, but those folders are left out of the total.',
  'access.steps': 'In System Settings, turn Cleany on in the Full Disk Access list. macOS may ask you to reopen the app.',
  'access.open': 'Open System Settings',
  'access.dismiss': 'Not now',
  'access.openError': 'Could not open System Settings. Go to Privacy & Security → Full Disk Access yourself.',

  'scan.title': (p) => `Scanning ${p.name}`,
  'scan.foundOf': (p) => `of roughly ${p.used}`,
  'scan.progressLabel': (p) => `${p.found} of ${p.used} found`,
  'scan.files': 'Files',
  'scan.dirs': 'Folders',
  'scan.elapsed': 'Time',
  'scan.status': (p) => `${p.found} found`,
  'scan.starting': 'Starting the scan…',
  'scan.cancel': 'Cancel',
  'scan.cancelHint': 'Esc cancels too. Scanning changes nothing.',

  'cancelled.title': 'Scan cancelled',
  'cancelled.body': 'The partial result was dropped. You can start again.',
  'failed.title': 'The scan could not finish',
  'failed.lead': 'An error stopped the scan partway; nothing was changed. The detail the system gave:',
  'retry': 'Scan again',
  'backToVolumes': 'Back to volumes',

  'summary.title': (p) => `${p.name} breakdown`,
  'summary.free': (p) => `${p.free} is free on the volume.`,
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
  'summary.noAccess': 'Without Full Disk Access, protected folders cannot be read.',
  'summary.stats': (p) => `${p.files} files and ${p.dirs} folders scanned in ${p.elapsed}.`,
  'summary.browseAll': 'Browse all',
  'summary.open': 'Open',
  'summary.openLabel': (p) => `Open: ${p.name}`,

  'browse.all': 'All',
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
  'browse.view': 'View',
  'browse.table': 'Table',
  'browse.map': 'Map',
  'browse.mapLabel': (p) => `Map of ${p.name}`,
  'browse.others': (p) => `${p.n} other ${items(p.count)}`,
  'browse.fileCount': (p) => `${p.n} ${p.count === 1 ? 'file' : 'files'}`,
  'browse.mapEmpty': 'Nothing in this folder takes up space to map.',
  'browse.mapHint': 'Arrows to pick a block, Enter or click to open, Backspace to go up, Esc for the breakdown.',
  'browse.rings': 'Rings',
  'browse.ringsLabel': (p) => `Rings of ${p.name}`,
  'browse.ringsEmpty': 'Nothing in this folder takes up space to show.',
  'browse.ringsHint': '← → to pick a slice, Enter or click to open, click the centre or Backspace to go up, Esc for the breakdown.',

  'actions.label': 'Selected item',
  'actions.target': (p) => `Selected: ${p.name}`,
  'actions.revealMac': 'Show in Finder',
  'actions.revealWin': 'Show in Explorer',
  'actions.revealOther': 'Show in file manager',
  'actions.open': 'Open',
  'actions.copy': 'Copy path',
  'actions.copied': 'Path copied.',
  'actions.gone': 'This item is gone. It may have been deleted or moved since the scan.',
  'actions.revealError': 'Could not open the file manager.',
  'actions.openError': 'Could not open this item.',
  'actions.copyError': 'Could not copy the path.',

  'ago.now': 'just now',
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
