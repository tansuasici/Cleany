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

  'volumes.title': 'Disk kullanımı',
  'volumes.lede': 'Taramak istediğiniz birimi seçin. Tarama yalnızca okur; hiçbir dosyayı değiştirmez veya silmez.',
  'volumes.loading': 'Birimler okunuyor…',
  'volumes.empty': 'Bağlı birim bulunamadı.',
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
  'volumes.lastScan': (p: { ago: string }) => `Son tarama: ${p.ago}. Çubuk en büyük kalemlere göre bölünmüştür.`,
  'access.title': 'Tam Disk Erişimi kapalı',
  'access.body':
    'macOS bazı klasörlere (Mail, Safari ve diğer uygulamaların verileri) erişimi kısıtlar. Tarama yine de çalışır, ancak bu klasörler toplama dahil edilmez.',
  'access.steps': 'Sistem Ayarları > Gizlilik ve Güvenlik > Tam Disk Erişimi bölümünde Cleany’yi etkinleştirin. macOS, uygulamayı yeniden başlatmanızı isteyebilir.',
  'access.open': 'Sistem Ayarları’nı aç',
  'access.dismiss': 'Şimdi değil',
  'access.openError': 'Sistem Ayarları açılamadı. Gizlilik ve Güvenlik > Tam Disk Erişimi bölümünü elle açabilirsiniz.',

  'scan.title': (p: { name: string }) => `${p.name} taranıyor`,
  'scan.foundOf': (p: { used: string }) => `yaklaşık ${p.used} içinden`,
  'scan.progressLabel': (p: { found: string; used: string }) => `${p.used} içinden ${p.found} bulundu`,
  'scan.files': 'Dosya',
  'scan.dirs': 'Klasör',
  'scan.elapsed': 'Süre',
  'scan.status': (p: { found: string }) => `${p.found} bulundu`,
  'scan.starting': 'Tarama başlatılıyor…',
  'scan.cancel': 'İptal et',
  'scan.cancelHint': 'Esc tuşuyla da iptal edebilirsiniz. Tarama hiçbir dosyayı değiştirmez.',

  'cancelled.title': 'Tarama iptal edildi',
  'cancelled.body': 'Tamamlanmamış sonuçlar kaydedilmedi. Taramayı yeniden başlatabilirsiniz.',
  'failed.title': 'Tarama tamamlanamadı',
  'failed.lead': 'Bir hata nedeniyle tarama durduruldu. Hiçbir dosya değiştirilmedi. Hata ayrıntısı:',
  'retry': 'Yeniden tara',
  'backToVolumes': 'Birimlere dön',

  'summary.title': (p: { name: string }) => `${p.name} dökümü`,
  'summary.free': (p: { free: string }) => `Birimde ${p.free} boş alan var.`,
  'summary.items': 'En büyük kalemler',
  'summary.share': 'Pay',
  'summary.childrenError': 'İçerik okunamadı.',
  'summary.loadingItems': 'Döküm hazırlanıyor…',
  'summary.itemsError': 'Döküm okunamadı.',
  'summary.noItems': 'Bu birimde yer kaplayan bir kalem bulunamadı.',
  'summary.others': (p: Count) => `Diğer ${p.n} öğe`,
  'summary.sum': 'Toplam',
  'summary.notCounted': 'Sayılmayanlar',
  'summary.hardlinks': (p: Count) =>
    `${p.n} sabit bağlantı (hard link) yalnızca bir kez sayıldı; kopyaları toplama eklenmedi.`,
  'summary.placeholders': (p: Count) =>
    `${p.n} iCloud/OneDrive yer tutucusu diskte yer kaplamadığı için sayılmadı; dosyalar indirilmedi.`,
  'summary.skippedMounts': (p: Count) =>
    `${p.n} başka birim (bağlama noktası) taramaya dahil edilmedi.`,
  'summary.unreadable': (p: Count) => `${p.n} öğe okunamadı.`,
  'summary.unreadableShow': (p: Count) => `İlk ${p.n} tanesini göster`,
  'summary.noAccess': 'Korumalı klasörleri okumak için Tam Disk Erişimi gereklidir.',
  'summary.stats': (p: { files: string; dirs: string; elapsed: string }) =>
    `${p.files} dosya ve ${p.dirs} klasör ${p.elapsed} sürede tarandı.`,
  'summary.browseAll': 'Tümünü incele',
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
  'browse.hint': 'Seçmek için ↑ ↓, açmak için Enter, üst klasöre çıkmak için ←, döküme dönmek için Esc.',
  'browse.view': 'Görünüm',
  'browse.table': 'Tablo',
  'browse.map': 'Harita',
  'browse.mapLabel': (p: { name: string }) => `${p.name} haritası`,
  'browse.others': (p: Count) => `Diğer ${p.n} öğe`,
  'browse.fileCount': (p: Count) => `${p.n} dosya`,
  'browse.mapEmpty': 'Bu klasörde haritada gösterilecek bir boyut yok.',
  'browse.mapHint': 'Seçmek için ok tuşları, açmak için Enter veya tıklama, üst klasöre çıkmak için Backspace, döküme dönmek için Esc.',
  'browse.rings': 'Halka',
  'browse.ringsLabel': (p: { name: string }) => `${p.name} halkaları`,
  'browse.ringsEmpty': 'Bu klasörde halkada gösterilecek bir boyut yok.',
  'browse.ringsHint': 'Seçmek için ← →, açmak için Enter veya tıklama, üst klasöre çıkmak için ortaya tıklama veya Backspace, döküme dönmek için Esc.',

  'actions.label': 'Seçili öğe',
  'actions.target': (p: { name: string }) => `Seçili: ${p.name}`,
  'actions.revealMac': "Finder'da göster",
  'actions.revealWin': "Gezgin'de göster",
  'actions.revealOther': 'Dosya yöneticisinde göster',
  'actions.open': 'Aç',
  'actions.copy': 'Yolu kopyala',
  'actions.copied': 'Yol kopyalandı.',
  'actions.gone': 'Bu öğe bulunamadı. Taramadan sonra silinmiş veya taşınmış olabilir.',
  'actions.revealError': 'Dosya yöneticisi açılamadı.',
  'actions.openError': 'Bu öğe açılamadı.',
  'actions.copyError': 'Yol kopyalanamadı.',
  'actions.trash': 'Çöpe taşı',
  'actions.trashRoot': 'Taranan birimin veya klasörün kendisi çöpe taşınamaz.',
  'trash.title': (p: { name: string }) => `“${p.name}” çöpe taşınsın mı?`,
  'trash.space': 'Boşalacak alan',
  'trash.folder': (p: { files: string }) => `Klasör · ${p.files}`,
  'trash.file': 'Dosya',
  'trash.undo': 'Öğeyi daha sonra çöpten geri yükleyebilirsiniz. Cleany taramayı bu öğe olmadan gösterir.',
  'trash.cancel': 'Vazgeç',
  'trash.confirm': 'Çöpe taşı',
  'trash.working': 'Taşınıyor…',
  'trash.done': (p: { name: string; size: string }) => `“${p.name}” çöpe taşındı. ${p.size} alan boşaldı.`,
  'trash.gone': (p: { name: string }) => `“${p.name}” bulunamadı; taramadan sonra silinmiş veya taşınmış olabilir. Listeden kaldırıldı.`,
  'trash.protected': (p: { name: string }) =>
    `“${p.name}” korunduğu için taşınmadı. Diskler, sistem klasörleri, ana klasör ve üstündeki klasörler, bağlama noktaları ve iCloud/OneDrive yer tutucusu içeren öğeler taşınamaz.`,
  'trash.failed': (p: { name: string; reason: string }) => `“${p.name}” çöpe taşınamadı: ${p.reason}`,
  'trash.error': 'Öğe çöpe taşınamadı.',

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

  'volumes.title': 'Disk usage',
  'volumes.lede': 'Choose a volume to scan. Scanning only reads; it never changes or deletes files.',
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
  'volumes.lastScan': (p) => `Last scan: ${p.ago}. The bar shows the largest items.`,
  'access.title': 'Full Disk Access is off',
  'access.body':
    'macOS restricts access to some folders (Mail, Safari and other apps’ data). Scans still run, but these folders are not included in the total.',
  'access.steps': 'In System Settings > Privacy & Security > Full Disk Access, turn on Cleany. macOS may ask you to restart the app.',
  'access.open': 'Open System Settings',
  'access.dismiss': 'Not now',
  'access.openError': 'Could not open System Settings. Open Privacy & Security > Full Disk Access manually.',

  'scan.title': (p) => `Scanning ${p.name}`,
  'scan.foundOf': (p) => `of roughly ${p.used}`,
  'scan.progressLabel': (p) => `${p.found} of ${p.used} found`,
  'scan.files': 'Files',
  'scan.dirs': 'Folders',
  'scan.elapsed': 'Time',
  'scan.status': (p) => `${p.found} found`,
  'scan.starting': 'Starting the scan…',
  'scan.cancel': 'Cancel',
  'scan.cancelHint': 'You can also press Esc to cancel. Scanning does not change any files.',

  'cancelled.title': 'Scan cancelled',
  'cancelled.body': 'Partial results were discarded. You can start the scan again.',
  'failed.title': 'The scan could not finish',
  'failed.lead': 'An error stopped the scan. No files were changed. Error details:',
  'retry': 'Scan again',
  'backToVolumes': 'Back to volumes',

  'summary.title': (p) => `${p.name} breakdown`,
  'summary.free': (p) => `${p.free} available on this volume.`,
  'summary.items': 'Largest items',
  'summary.share': 'Share',
  'summary.childrenError': 'Could not read the contents.',
  'summary.loadingItems': 'Preparing the breakdown…',
  'summary.itemsError': 'Could not read the breakdown.',
  'summary.noItems': 'No items taking up space were found on this volume.',
  'summary.others': (p) => `${p.n} other ${items(p.count)}`,
  'summary.sum': 'Total',
  'summary.notCounted': 'Not counted',
  'summary.hardlinks': (p) =>
    `${p.n} ${hardlinks(p.count)} counted only once; the copies are not added to the total.`,
  'summary.placeholders': (p) =>
    `${p.n} iCloud/OneDrive ${placeholders(p.count)} no disk space and were not counted; nothing was downloaded.`,
  'summary.skippedMounts': (p) => `${p.n} ${volumes(p.count)} left out of this scan.`,
  'summary.unreadable': (p) => `${p.n} ${entries(p.count)} could not be read.`,
  'summary.unreadableShow': (p) => `Show the first ${p.n}`,
  'summary.noAccess': 'Full Disk Access is required to read protected folders.',
  'summary.stats': (p) => `${p.files} files and ${p.dirs} folders scanned in ${p.elapsed}.`,
  'summary.browseAll': 'Explore all',
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
  'browse.hint': '↑ ↓ to select, Enter to open, ← to go up, Esc to return to the breakdown.',
  'browse.view': 'View',
  'browse.table': 'Table',
  'browse.map': 'Map',
  'browse.mapLabel': (p) => `Map of ${p.name}`,
  'browse.others': (p) => `${p.n} other ${items(p.count)}`,
  'browse.fileCount': (p) => `${p.n} ${p.count === 1 ? 'file' : 'files'}`,
  'browse.mapEmpty': 'Nothing in this folder takes up space to map.',
  'browse.mapHint': 'Arrow keys to select, Enter or click to open, Backspace to go up, Esc to return to the breakdown.',
  'browse.rings': 'Rings',
  'browse.ringsLabel': (p) => `Rings of ${p.name}`,
  'browse.ringsEmpty': 'Nothing in this folder takes up space to show.',
  'browse.ringsHint': '← → to select, Enter or click to open, click the center or press Backspace to go up, Esc to return to the breakdown.',

  'actions.label': 'Selected item',
  'actions.target': (p) => `Selected: ${p.name}`,
  'actions.revealMac': 'Show in Finder',
  'actions.revealWin': 'Show in Explorer',
  'actions.revealOther': 'Show in file manager',
  'actions.open': 'Open',
  'actions.copy': 'Copy path',
  'actions.copied': 'Path copied.',
  'actions.gone': 'This item could not be found. It may have been deleted or moved since the scan.',
  'actions.revealError': 'Could not open the file manager.',
  'actions.openError': 'Could not open this item.',
  'actions.copyError': 'Could not copy the path.',
  'actions.trash': 'Move to Trash',
  'actions.trashRoot': 'The scanned volume or folder itself cannot be moved to the trash.',
  'trash.title': (p) => `Move “${p.name}” to the trash?`,
  'trash.space': 'Space freed',
  'trash.folder': (p) => `Folder · ${p.files}`,
  'trash.file': 'File',
  'trash.undo': 'You can restore it from the trash later. Cleany shows the scan without it.',
  'trash.cancel': 'Cancel',
  'trash.confirm': 'Move to Trash',
  'trash.working': 'Moving…',
  'trash.done': (p) => `Moved “${p.name}” to the trash. ${p.size} freed.`,
  'trash.gone': (p) => `“${p.name}” could not be found; it may have been deleted or moved since the scan. It was removed from the list.`,
  'trash.protected': (p) =>
    `“${p.name}” is protected and was not moved. Disks, system folders, the home folder and the folders above it, mount points, and items holding an iCloud/OneDrive placeholder cannot be moved.`,
  'trash.failed': (p) => `Could not move “${p.name}” to the trash: ${p.reason}`,
  'trash.error': 'Could not move the item to the trash.',

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
