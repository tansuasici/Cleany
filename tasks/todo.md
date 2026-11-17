# Task Board

Track current and upcoming tasks here. The agent updates this file as work progresses.

Give the task you are actively working on an `h3` heading under **In Progress** —
`session-start.sh` reads the first one and injects it as the session's active task.

---

## In Progress

### M4 — Kalıcı silme: ayardan açılır, yazarak onaylanır (Notion #31) — uygulandı, smoke test geçti

**Durum (2026-11-17)**: Uygulandı, commit edilmedi. Ayrıntı aşağıda "Durum" başlığında. Dal: `feat/permanent-delete` (main a3b4a12'den).

**Hedef**: Kullanıcı ayarlardan açarsa, seçili öğeyi ya da sepeti çöpe uğratmadan diskten silebilsin. Silme, sabit bir sözcük yazılmadan onaylanamasın. Ayar kapalıyken arayüzde kalıcı silmenin izi bile olmasın. Çöpe taşıma varsayılan olarak kalır (ADR-0004).

**Bağlam (bugün)**:
- Akış: `trash_nodes` → `actions::trash` → `Sessions::trash(scan, nodes, mover)` → `(TrashReport, Vec<Deletion>)` → `deletion_log::append_trash`.
- `plan_trash` kökü ve bulut yer tutucusu taşıyan klasörleri mover'a hiç göndermez. `move_one` sırayla şunlara bakar: diskte yoksa `Gone`, `Protection::covers` ise `Protected` (bağlama noktası **içeren** klasör dahil, bu yüzden özyinelemeli silme başka bir birime inemez), türü değiştiyse `Failed`.
- Günlük `action:"trash"` ve `to:"osTrash"` değerlerini sabit yazıyor. #32 planı #31 için `delete` ve `permanent` değerlerini ayırmıştı.
- Arayüzde ayar ekranı yok. Kalıcı tercihler `localStorage` içinde `cleany.*` anahtarlarıyla tutuluyor (`cleany.locale`, `cleany.accessDismissed`).

#### Tasarım kararları
1. **Ayrı komut**: `delete_nodes(scan, nodes)`. `trash_nodes` için bir kip parametresi eklemiyoruz, böylece unutulmuş bir varsayılan hiçbir zaman "kalıcı" anlamına gelemez.
2. **Platform işlevi**: `cleany_platform::trash::delete_permanently(path) -> Result<(), DeleteError>` (`Gone | Failed(String)`, `TrashError` ile aynı biçimde). Davranışı:
   - `symlink_metadata` ile bakar, bağlantıyı asla izlemez. Gerçek klasör `fs::remove_dir_all` ile silinir. Bu işlev std 1.58.1'den beri sembolik bağlantıları izlemez ve TOCTOU'ya karşı korumalıdır.
   - Dosya ve bağlantı `fs::remove_file` ile silinir. Windows'ta klasör bağlantısı ve junction `fs::remove_dir` ile silinir.
   - Salt okunur dosyanın özniteliğine dokunmaz. İşletim sistemi reddederse sonuç `Failed` olur.
   - Yeni bağımlılık yok.
3. **Ortak ön kontroller**: `move_one` içindeki gone, protected ve kind kontrolleri `precheck` adlı bir işleve çıkar. `move_one` (çöp) ve yeni `remove_one` (kalıcı) ikisi de bunu kullanır.
4. **Sonuç türü**: `TrashOutcome::Deleted` (IPC: `{ kind: 'deleted' }`). Değişen yerler:
   - `Sessions::trash` içinde `gone_from_disk` ve sayaç;
   - `outcome_name` → `"deleted"`;
   - `ipc.ts` birleşimi;
   - `Browse.tsx` içindeki `describe()`.
   
   Rapordaki `trashed` sayısı "bu istekle diskten kalkan" anlamına genişler, alan adı aynı kalır ve doc güncellenir. `Sessions::trash`, ağacı aynı şekilde güncelleyip `edits` sayacını artırdığı için canlı izleme (#35) değişmeden uyumlu kalır.
5. **Yarım kalan silme**: `remove_dir_all` yarıda hata verirse öğe `Failed` olur, ağaçta kalır ve `freed` sayılmaz. İçinden silinenleri canlı izleyici (FSEvents) yamalar, klasörün boyutu kendiliğinden küçülür. Rapor metni bunu dürüstçe söyler: "bir kısmı silinmiş olabilir". Bu durum smoke testte kilitli bir dosyayla denenecek.
6. **Günlük**: `append_trash` yerine `append(dir, Removal, &deletions)` gelir, `enum Removal { Trash, Delete }` ile. `Trash` → `trash` / `osTrash`. `Delete` → `delete` / `permanent`.
7. **Ayar**: `localStorage` → `cleany.permanentDelete = '1'`, store'da `permanentDelete` ve `setPermanentDelete`. Varsayılan kapalı. **Arka uç ayarı denetlemez.** Komutu yalnız kendi webview'imiz çağırabildiği için (#34: boş izin listesi ve sıkı CSP) bu kabul edilebilir. İstenirse ayar Rust tarafına taşınabilir (ek bir komut ve bir dosya gerekir).
8. **Ayar yeri**: başlıkta dil menüsünün yanına bir "Ayarlar" düğmesi gelir. Bu düğme bir `<dialog>` açar, içinde tek bir onay kutusu bulunur: "Kalıcı silmeyi etkinleştir". Altında dipnot: "Açıkken öğeleri çöpe taşımadan silebilirsiniz. Kalıcı olarak silinen öğeler geri alınamaz." Açarken ek bir onay sorulmaz, çünkü asıl onay her silmede yazılarak verilir.
9. **Eylemler (ayar açıkken)**:
   - `ActionBar` içinde "Kalıcı olarak sil" düğmesi. Kısayol macOS'ta ⌥⌘⌫ (Finder'ın "Hemen sil" kısayolu), diğerlerinde Shift+Delete. `trashKey` Alt ve Shift'i zaten yok saydığı için çakışma yok.
   - `BasketBar` içinde ikincil "Kalıcı olarak sil" düğmesi.
   - Kök için devre dışıdır, tıpkı Çöpe taşı gibi.
10. **Onay penceresi**: `TrashConfirm` bir `removal: 'trash' | 'delete'` alır.
    - `delete` kipinde "geri alabilirsiniz" satırı çıkar. Yerine şu gelir: "Bu işlem geri alınamaz. Öğe çöpe gitmez, diskten hemen silinir."
    - Bir giriş alanı eklenir: "Onaylamak için **sil** yazın" (en: **delete**). Tek öğe ve sepet için aynı sözcük kullanılır, çünkü uzun ya da Unicode adların yazılması zor.
    - Karşılaştırma: `trim()` + `toLocaleLowerCase(locale)`, Türkçe i/İ/ı/I için.
    - Sözcük tutana kadar "Kalıcı olarak sil" devre dışı kalır. Enter yalnız eşleşince gönderir.
    - Odak giriş alanına gider. Escape vazgeçer.
    - Düğme `primary` kalır, kırmızı kullanılmaz (Don'ts: korkutma yok, negatif renk yalnız gerçek hata için).
11. **Metin**: tr ve en dilleri için yeni anahtarlar:
    - `delete.*`: başlık, uyarı, yazma etiketi, düğme, çalışıyor, bitti ("“x” kalıcı olarak silindi. 1,2 GB alan boşaldı."), başarısız, yarım;
    - `settings.*`.

    Metinler "siz" hitabıyla yazılır, noktalı virgül ve tire kullanılmaz.

#### Onay gerekenler
- Yeni genel API: `cleany_platform::trash::delete_permanently` ve `DeleteError`.
- IPC: `delete_nodes` komutu ve `TrashOutcome` içine `deleted` varyantı.
- Ayarın yalnız `localStorage` içinde tutulması, arka ucun ayarı denetlememesi (karar 7).
- Paylaşılan arayüz: başlıktaki yeni Ayarlar düğmesi ve penceresi, `TrashConfirm` değişikliği.
- Bağımlılık, `Cargo.toml`, capability ya da CSP değişikliği **yok**.

#### Testler
- Platform (`temp_dir` içindeki karalama klasörlerinde):
  - dosya ve iç içe klasör siliniyor;
  - klasöre işaret eden bir bağlantı silinince hedef yerinde kalıyor;
  - olmayan yol `Gone` dönüyor.
- `actions`: `remove_one` kök ve tür uyuşmazlığında hiçbir şey silmeden `Protected` ya da `Failed` dönüyor.
- `session`: `Deleted` sonucu ağaçtan çıkarıyor, sayıyor ve `freed` değerine ekliyor.
- `deletion_log`: `action:"delete"`, `to:"permanent"`, `outcome:"deleted"`.
- `ipc_ts_mentions_every_action_tag` testine `'delete_nodes'` eklenir.
- Arayüz: `pnpm typecheck` + `build`.

#### Dosyalar
- `crates/cleany-platform/src/trash.rs`
- `src-tauri/src/actions.rs`, `session.rs`, `deletion_log.rs`, `lib.rs`
- `ui/src/lib/ipc.ts`, `lib/i18n.ts`, `store.ts`, `App.tsx`
- `screens/Browse.tsx`, `BrowseActions.tsx`, `Basket.tsx`, `useBrowseActions.ts`
- yeni `components/SettingsDialog.tsx`
- `CODEBASE_MAP.md`

#### Doğrulama
- `cargo fmt --check`, `clippy --all-targets -D warnings`, `check`, `test`, `build --release`.
- Windows ve Linux için çapraz hedef clippy.
- Arayüz için ekran görüntüleri: 375, 768 ve 1280 genişlikte, açık ve koyu tema, yalnız klavyeyle.
- Smoke test **yalnız `~/cleany-trash-test` içinde ve kullanıcıya sorduktan sonra** yapılır:
  - tek dosya ve sepetle kalıcı silme;
  - `deletions.jsonl` satırlarını kontrol etme;
  - ayar kapalıyken düğmenin ve kısayolun yok olduğunu görme.

#### Kararlar (2026-11-17)
- Ayar yeri: başlıkta ayrı bir Ayarlar penceresi.
- Yazılacak sözcük: tek öğede de sepette de sabit "sil" / "delete".

#### Durum (2026-11-17)
- Rust ve UI kontrolleri temiz: fmt, clippy (Windows ve Linux çapraz dahil), test, release derlemesi, typecheck, lint, build.
- Arayüz sahte IPC ile tarayıcıda denendi: 375, 768 ve 1280 genişlik, açık ve koyu tema, yalnız klavye. Boş veya eksik sözcükle Enter onaylamıyor, "SİL" kabul ediliyor, Escape kapatıyor ve odak düğmeye dönüyor, ayar kapalıyken düğmeler ve ⌥⌘⌫ devre dışı.
- Düzeltme: Ayarlar penceresi kapanınca odak gövdeye düşüyordu, artık pencere kapandıktan sonra Ayarlar düğmesine dönüyor.
- Dil menüsü kaldırıldı (`LocaleMenu.tsx` silindi), dil seçimi Ayarlar penceresine taşındı (radyo düğmeleri, her dil kendi adıyla). Tarayıcıda tr/en, Escape ve odak dönüşü, 375 genişlik ve koyu tema denendi.
- Smoke test (debug paket, `~/cleany-trash-test`, yalnız klavye) geçti:
  - Tek dosya ⌥⌘⌫ ile silindi. Boş alanla Enter onaylamadı, "Delete" (büyük harf) kabul edildi. Dosya Çöp'e gitmedi.
  - Sepetteki 3 öğe (2 dosya ve bir klasör) sepet çubuğundaki düğmeyle silindi, 3.0 MB.
  - `locked` (önce kilitli `inner`): `remove_dir_all` ilk hatada durdu, hiçbir şey silinmedi, Failed döndü, ağaçta aynen kaldı.
  - `locked2` (önce silinebilir dosya, sonra kilitli klasör): Failed döndü, ağaçta kaldı, izleyici 8.2 KB / 2 dosyadan 4.1 KB / 1 dosyaya yamaladı.
  - `deletions.jsonl` her satırda `"action":"delete"`, `"to":"permanent"` ve doğru `outcome` ile geçerli JSONL.
- Gözlem (Not Now): dar pencerede eylem çubuğundaki "Kalıcı olarak sil" ikinci satıra iniyor. Yeniden sıralanan tabloda seçim, silinmek istenen satırdan ilk satıra kayıyor.

#### Riskler
- Windows'ta junction ve salt okunur dosya davranışı CI'da yalnız derlenerek doğrulanır, gerçek silme denenmez.
- Yarım kalan klasör silmesi geri alınamaz. Ön kontroller ve yazarak onay bu yüzden zorunlu.

## Up Next

_(boş)_

---

## Done

### M4 — Tauri yetkilerini daraltma + CSP (Notion #34) ✅ 2026-11-17 (#36 merged)

**Durum (2026-11-17)**: Yalnız plan. `src-tauri/capabilities/` ve `tauri.conf.json` korumalı dosyalar, onay gerekiyor. Dal: `feat/capabilities-csp`.

**Hedef**: Arayüz yalnız kendi komutlarımızı çağırabilsin. Webview'e yüklenecek içerik yalnız uygulamanın kendi dosyaları olsun. Bir XSS olsa bile pencere, dosya yolu, menü gibi çekirdek API'ler ve dış ağ kapalı kalsın.

**Bağlam (bugün)**:
- `capabilities/default.json` → `core:default`. Bu set path, event, window, webview, app, image, resources, menu, tray için varsayılan izinleri açıyor.
- Arayüz bunların hiçbirini kullanmıyor. Tek kullanılan `@tauri-apps/api/core`: `invoke` ve `Channel`.
- Kendi 15 komutumuz (`generate_handler!`) `build.rs` içinde app manifest olmadığı için izin listesine tabi değil. Bu değişiklikle de öyle kalıyor.
- CSP: `default-src 'self'; img-src 'self' data: asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost`
  - `asset:` protokolü kullanılmıyor (`convertFileSrc` yok, `assetProtocol` kapalı).
  - `object-src`, `base-uri`, `form-action`, `frame-src` tanımlı değil, `default-src`'a düşüyor ya da hiç sınırlanmıyor (`base-uri`, `form-action` düşmez).

#### Değişiklikler
1. `src-tauri/capabilities/default.json`: `"permissions": []`. Yalnız `main` penceresi.
   - Kanal (`Channel`) mesajları iç `__TAURI_CHANNEL__` eklentisinden geçer, izin istemez. Smoke testte doğrulanacak. Gerekirse en dar izin (`core:event:default` gibi) tek tek eklenir, nedeni buraya yazılır.
2. `src-tauri/tauri.conf.json` → `app.security`:
   - `csp`: `default-src 'self'; script-src 'self'; style-src 'self'; style-src-attr 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'; worker-src 'none'`
     - `asset:` ve `http://asset.localhost` çıkar.
     - `style-src` içinden `'unsafe-inline'` çıkar. React'in `style={{…}}` öznitelikleri (çubuk genişlikleri, transform) için yalnız `style-src-attr 'unsafe-inline'` kalır. Tauri `style-src`'a nonce eklediğinde `'unsafe-inline'` zaten yok sayılırdı. Bu ayrım onu da netleştirir.
   - `freezePrototype: true`: `Object.prototype` dondurulur, prototip kirletme yoluyla IPC'ye müdahale kapanır.
3. `tasks/decisions.md`: kısa bir ADR ("Webview yetkileri: boş izin listesi + sıkı CSP"), bundan sonra yeni bir çekirdek API ya da eklenti gerekirse izni dar ekleme kuralıyla.

Rust kodu, IPC, bağımlılık, arayüz değişikliği yok.

#### Doğrulama
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets`, `cargo test`.
- CSP dev modunda uygulanmıyor (sayfa Vite'tan doğrudan yükleniyor). Bu yüzden asıl test `pnpm tauri build` ile üretilen .app üzerinde:
  - Açılış, birim listesi, tarama ilerlemesi (Channel), döküm, tablo, harita ve halka (canvas), sepet, çöpe taşı, canlı güncelleme.
  - Web Inspector konsolunda CSP ihlali olmamalı.
- Dev uygulama (`tauri dev`) eskisi gibi açılmalı.

### Gezinti — Browse başlığında Geri / İleri ✅ 2026-11-17 (#34 merged)

**Durum (2026-11-16)**: Uygulandı, commit edilmedi. typecheck, oxlint, build temiz; 375/768/1280 açık+koyu ekran görüntüleri ve klavye (Alt+←/→, Ctrl+[/], düz ← üst klasör) sahte IPC ile tarayıcıda doğrulandı. Masaüstü uygulamada ⌘[ elle denenmedi. Dal: `feat/browse-history` (main'den; `feat/live-session` ile çakışmayı küçük tutmak için ayrı worktree).

**Hedef**: Browse ekranındaki klasör başlığının ("ad ······ boyut") soluna tarayıcıdaki gibi Geri / İleri düğmeleri.

#### Tasarım kararları
- **Durum `store.ts` içinde**: `history: { scan, entries: NodeEntry[], at: number } | null`.
  - `browse(node)` geçmişe yazar: `entries[at]` ile aynı klasörse yerinde tazeler; değilse `at`'tan sonrasını atar (ileri yolu silinir) ve yeni klasörü ekler.
  - Dökümden tabloya geçiş geçmişi silmez; aynı taramada süren bir gezinti sayılır.
  - Yeni tarama (`scan`) geçmişi siler; `scan` alanı farklı bir taramaya ait geçmişi zaten geçersiz kılar.
- **`go(step)` eylemi** (−1 geri, +1 ileri): hedef klasörü `getAncestors` ile tazeler. Çöpe taşınıp ağaçtan çıkmış bir kayıt geçmişten düşer ve bir sonrakine geçilir. Bekleyen yanıt, araya giren başka bir gezinti varsa uygulanmaz.
- **Düğmeler**: mevcut `Button` (ikincil), yalnız simge (`ChevronIcon`, geri için yatay çevrilmiş), `aria-label`, `title` ve `aria-keyshortcuts`. Geçmiş yoksa `disabled`. Yeni simge, token veya animasyon yok.
- **Kısayollar**: macOS'ta ⌘[ / ⌘], diğer sistemlerde Ctrl+[ / Ctrl+]; her yerde Alt+← / Alt+→. Tuş konumu (`e.code`) okunur, Türkçe klavyede de çalışır. Yakalama (capture) aşamasında bağlanır; görünümler Alt+← tuşunu "üst klasör" olarak okumaz. Değiştiricisiz ← üst klasör davranışı aynen kalır.
- **Geri gelinen klasör**: geçmişte yer değiştirince bırakılan klasör, yeni listede varsa seçili gelir (`cameFrom`).

#### Dosyalar
- `ui/src/store.ts`: `history`, `browse` içinde kayıt, `go`, `scan` içinde temizleme.
- `ui/src/screens/Browse.tsx`: düğmeler ve kısayol.
- `ui/src/lib/i18n.ts`: `browse.back`, `browse.forward`, `browse.history` (TR + EN).

#### Doğrulama
- `pnpm --dir ui typecheck`, `pnpm --dir ui lint`, `pnpm --dir ui build`.
- 375 / 768 / 1280 genişlikte, açık ve koyu temada ekran görüntüsü.

### M4 — Seçim sepeti + toplam kazanılacak alan (Notion #33) ✅ 2026-11-16 (#29 merged)

**Durum (2026-11-16)**: Uygulandı, PR açıldı. typecheck, lint ve build temiz. Çalışan uygulamada elle denenmedi.

**Hedef**:
- Birden çok öğe toplanıp tek onayla çöpe taşınabilir.
- Sepet her an toplam kazanılacak alanı gösterir.
- docs/PLAN.md:44 ve :61: "seç → sepete ekle → 'Çöpe taşı' onayı".

**Bağlam**:
- Backend zaten çoklu öğeyi destekliyor: `trash_nodes(scan, nodes[])`. İç içe istenen öğeler raporda tekrar sayılmaz.
- Bu yüzden iş yalnız arayüzde. Rust, IPC ve yetenek (capability) değişikliği yok.
- Dal: `feat/m4-basket`.

#### Tasarım kararları
- **Durum `store.ts` içinde tutulur.**
  - Yapı: `basket: { scan, items: BasketItem[] } | null`.
  - `BasketItem` alanları: `id`, `name`, `kind`, `allocated` ve `chain` (atalarının id'leri).
  - Yeni tarama sepeti boşaltır. Tablo ile döküm arasında geçiş sepeti korur.
- **Ekleme ve çıkarma.**
  - Ekle/çıkar düğmesi işlem çubuğunda (ActionBar) durur. Klavye kısayolu Space'tir ve üç görünümde de çalışır.
  - Seçili öğeye, seçim yoksa ekrandaki klasöre uygulanır.
  - Taramanın kökü eklenemez.
  - `chain` ve güncel boyut eklenirken `getAncestors` ile alınır.
- **Çakışma: aynı alan iki kez sayılmaz.**
  - Sepetteki bir klasörün altındaki öğe eklenmez. Durum satırında "“X” zaten sepetteki “Y” klasörüne dahil." yazar.
  - Sepetteki öğeleri içeren bir klasör eklenirse o öğeler sepetten çıkar ve klasör eklenir. Durum satırında bu söylenir.
  - Toplam, öğelerin `allocated` toplamıdır. Tasarım gereği çakışma olmaz.
  - Öğeler arasındaki hardlink'ler toplamda fazla sayılabilir. Gerçekten boşalan alanı çöpe taşıma sonrası rapor söyler.
- **İşaret.**
  - Tablodaki satırda, adın önünde küçük bir sepet işareti görünür. Ekran okuyucuya "sepette" denir.
  - Harita ve halka görünümlerinde işaret yok. Seçim satırında "Sepette" yazar. Bu bilinçli bir kapsam sınırı.
- **Sepet şeridi.**
  - İşlem çubuğunun altında durur ve yalnız sepet doluyken görünür.
  - İçeriği: "Sepet · 3 öğe · Kazanılacak alan 12,4 GB", "Sepeti çöpe taşı", "Boşalt" ve "Listeyi göster".
  - Liste her öğe için adını ve boyutunu (LeaderRow) ve bir "Çıkar" düğmesini gösterir.
- **Onay penceresi.**
  - `TrashDialog` birden çok öğe alacak şekilde genelleşir.
  - Başlık: "3 öğe çöpe taşınsın mı?".
  - Gövde: en fazla 5 satır, kalanlar için "+k öğe daha" ve en altta "Boşalacak alan" toplamı.
  - Tek öğe akışı (⌘⌫ / Delete) olduğu gibi kalır.
- **Çöpe taşıma sonrası.**
  - Rapor özetlenir: "3 öğe çöpe taşındı. X alan boşaldı." Taşınamayanlar için uyarılar eklenir.
  - Taşınan ve bulunamayan öğeler sepetten çıkar. Korumalı ve hatalı olanlar sepette kalır.
  - Tek öğe çöpe taşınınca, sepette o öğe veya altındakiler varsa onlar da sepetten düşer.

#### Dosyalar
- `ui/src/store.ts`: sepet durumu ve eylemleri.
- `ui/src/screens/Basket.tsx` (yeni): sepet şeridi ve listesi.
- `ui/src/screens/BrowseActions.tsx`: ekle/çıkar düğmesi, çok öğeli `TrashDialog`.
- `ui/src/screens/useBrowseActions.ts`: Space kısayolu.
- `ui/src/screens/Browse.tsx`: bağlantılar ve satır işareti.
- `ui/src/lib/i18n.ts`: TR ve EN metinleri.
- `CODEBASE_MAP.md`: gerekirse bir satır.

#### Doğrulama
- `pnpm --dir ui typecheck && pnpm --dir ui lint && pnpm --dir ui build`.
- Rust tarafı değişmediği için Rust kontrolleri yalnız güvence olarak çalışır.
- `tauri dev` içinde elle kontrol: ekle, çakışma, Space, onay, rapor, tek öğe silmenin sepeti budaması.

### Canlı tarama: biten taramayı yeniden taramadan güncel tut (FSEvents) ✅ 2026-11-17 (#35 merged)

**Durum (2026-11-16)**: Yalnız plan. Kod yok. Yeni bağımlılık, IPC sözleşmesi ve yeni `pub` API için onay gerekiyor (aşağıda "Onay gereken değişiklikler").

**Hedef**: Tarama bittikten sonra kökün altında uygulama dışında yapılan bir değişiklik (dosya ekleme, silme, büyüme) "Tara"ya basmadan birkaç saniye içinde ağaçta, tabloda, haritada ve sepette görünür. Arayüz yeniden yüklenince (Vite HMR) tarama kaybolmaz.

**Bağlam**:
- Bugün tarama bellekte bir anlık görüntü (`src-tauri/src/session.rs` → `State::Done { Finished { tree } }`).
- Uygulama içinden çöpe taşıma ağacı `Tree::detach` ile günceller. Dışarıdaki değişiklik ise ancak yeniden taramayla görünür.
- `on_event` kanalı tarama iş parçacığına taşınır ve `done` olayından sonra düşer. Yani bugün `done` sonrası UI'a olay taşıyacak bir yol yok.
- Kullanıcı şunu sordu: "Neden her seferinde yeniden Tara'ya basmam gerekiyor?"
- `docs/PLAN.md:56` canlı izlemeyi v2'ye koyuyor. Bu plan onu öne çekiyor; onay bunu da kapsıyor.

#### "Dev yeniden başlatma" iki ayrı durum
1. **Yalnız webview yenilenir** (Vite HMR, ⌘R): Rust süreci ve ağaç yaşıyor. Ama `store.ts` sıfırlanıp "Birimler" ekranına dönüyor.
   - Çözüm ucuz: yeni `attach_scan` komutu (aşağıda) güncel `Done` taramanın özetini ve kimliğini döner. UI açılışta bunu çağırıp kaldığı taramaya bağlanır.
   - Disk önbelleği gerekmez.
2. **Rust yeniden derlenir ve süreç yeniden başlar** (`tauri dev` içinde Rust değişikliği; üretimde uygulamayı kapatıp açmak): bellekteki her şey gider.
   - Bunu yalnız disk önbelleği çözer. Kararı aşağıda.

#### Tasarım kararları
- **İzleyici nerede: `cleany-platform`; yaşam döngüsü `src-tauri`'de.**
  - `crates/cleany-platform/src/watch.rs`: OS izleyicisinin ince sarmalayıcısı. `notify` olaylarını platformdan bağımsız `FsChange { path, kind: Entry | Subtree }` değerlerine çevirir.
    - `Subtree`, FSEvents'in `MustScanSubDirs` / olay kaybı bayraklarından gelir.
    - Platform crate'i core'a bağlanmaz (ADR-0005).
  - `crates/cleany-core/src/refresh.rs`: saf mantık.
    - Değişen yolları birleştirip bir yenileme planına çevirir.
    - Bir klasörü yeniden listeleyip ağaca yamar.
    - Dosya sistemine dokunmadan birim testi yazılabilir.
  - `src-tauri/src/session.rs`: izleyiciyi `State::Done` içinde tutar, birleştirme iş parçacığını çalıştırır, `changed` olayını yollar.
    - Taramanın yerini yeni tarama alınca izleyici düşer ve durur.
  - `unsafe_code = "deny"` olduğundan FSEvents mutlaka bir crate üzerinden gelir. Kendi FFI'ımızı yazmayız.
- **Platformlar.**
  - İlk sürüm yalnız macOS (FSEvents): `#[cfg(target_os = "macos")]`.
  - Windows ve Linux'ta `watch` bir no-op döner; davranış bugünkü gibi kalır.
  - Linux'ta `inotify` her klasöre ayrı izleme ister. Büyük ağaçlarda `max_user_watches` sınırına takılır. Bu yüzden ayrı iş.
- **İzleme taramayla birlikte başlar.**
  - Değişiklikler `done` gelene kadar tamponda birikir, sonra uygulanır. Yoksa tarama sürerken yapılan değişiklikler kaybolur.
  - Tampon bir sınırı aşarsa (ör. 10 000 yol) tek bir "kök yeniden taransın" işaretine düşer.
- **Birleştirme (coalescing).**
  - Olaylar sessizlik penceresiyle toplanır: son olaydan 1 sn sonra işlenir, en geç 5 sn'de bir zorla işlenir.
  - Her yol, ebeveyn klasörüne çevrilir: "bu klasörü yeniden listele".
  - Kökün dışındaki yollar atılır.
  - Bir `Subtree` işaretinin altında kalan klasörler atılır.
  - Aynı ebeveyni paylaşan çok sayıda klasör (ör. > 256) ortak ataya yükseltilir.
  - Kök için `Subtree` gelirse tam yeniden tarama yapılır ve yeni `ScanId` alınır. UI bunu bugünkü tarama akışıyla gösterir.
- **Yalnız etkilenen kısım yeniden taranır; düğüm kimlikleri korunur.**
  - Varsayılan "sığ yenileme"dir. Klasör tek seviye listelenir ve mevcut çocuklarla adına ve türüne göre eşleştirilir.
    - Eşleşen dosya: boyutu ve `mtime` değeri yerinde güncellenir. Fark ataların toplamına eklenir. Kimlik aynı kalır.
    - Eşleşen alt klasör: dokunulmaz. Onun kendi değişikliği kendi olayıyla gelir.
    - Kaybolan girdi: `Tree::detach`.
    - Yeni dosya: arenanın sonuna eklenir.
    - Yeni alt klasör: mevcut tarayıcıyla (`scan`) kendi alt ağacı taranır ve aşılanır.
  - `Subtree` işareti gelirse o klasörün çocukları detach edilir ve alt ağaç baştan taranıp aşılanır.
  - Böylece UI'ın tuttuğu `u32` kimlikler (açık klasör, `basket.chain`, seçim) normal değişikliklerde geçerli kalır.
  - Arena "çocuk, ebeveynden büyük indekste" kuralını korur, çünkü yeni düğümler hep sona eklenir.
- **Yoldan düğüme.**
  - `Tree` bugün yalnız düğümden yola gidiyor.
  - Yeni `Tree::lookup(path)` kökten başlayıp bileşen bileşen çocuklar arasında ada göre arar. Maliyeti her bileşende kardeş sayısıyla doğrusaldır.
  - Sığ yenileme, yenilenen klasör için zaten geçici bir `ad → id` haritası kurar. 100 000 çocuklu bir klasör bir kez taranır.
- **Tarayıcı kuralları alt ağaçta da geçerli.**
  - `scan` iki seçenek alır: ilk taramanın kök aygıtı (`root_dev`) ve `cross_mounts`.
  - Böylece alt ağaç taraması bağlama noktası sınırını orijinal köke göre uygular.
  - Sembolik bağlar izlenmez. Yer tutucular indirilmez (tarayıcının mevcut mantığı aynen kullanılır).
- **Hardlink'ler yaklaşık.**
  - Tekilleştirme bugün tüm tarama için `finish` adımında yapılıyor. Alt ağaç taraması yalnız kendi içinde tekilleştirir.
  - Ağacın başka yerindeki bir dosyaya hardlink olan yeni dosya tam boyutuyla sayılır. `detach` için de durum bugün aynı; belgelenir.
- **Kilit disiplini `trash` ile aynı.**
  - Disk G/Ç (listeleme, alt ağaç taraması) kilit dışında yapılır.
  - Kilit yeniden alınınca `ScanId` hâlâ güncel ve `Done` mı diye bakılır. Değilse yama atılır.
  - `sorted` önbelleği temizlenir.
- **Uygulamanın kendi çöpe taşımaları.**
  - FSEvents bunları da bildirir. Düğüm zaten detach edilmiş olduğundan yeniden listeleme değişiklik bulmaz.
  - Yama idempotent olduğu için bunlar no-op olur.
- **Arena büyümesi sınırlı.**
  - Detach edilen düğümlerin yerleri boşalmaz.
  - Ölü düğüm sayısı canlı düğüm sayısını geçerse yama yerine tam yeniden tarama yapılır (yeni `ScanId`).
  - Sıkıştırma (kimlikleri yeniden numaralamak) bu işte yok.
- **Yük.**
  - Sürekli değişen klasörler (`~/Library/Caches`, derleme `target/`, tarayıcı profilleri) sürekli yenileme tetikleyebilir.
  - Birleştirme penceresi bunu en çok 5 sn'de bir yenilemeye indirir.
  - Pencere gizliyken izleme duraklatılabilir. Bu bir açık soru.

#### UI'a iletim (yeni IPC)
- **`ScanEvent::Changed`** (yeni varyant). Alanlar:
  - `revision: u32`: her yamada artar.
  - `root: NodeEntry`: güncel kök.
  - `dirs: u32[]`: içeriği değişen klasörler, en çok 256 tane. Sınır aşılırsa `truncated: true` gelir.
  - Olay küçük kalır. Ağaç gönderilmez (CLAUDE.project.md: "IPC: send tree slices").
  - Birleştirme nedeniyle 1 Hz'in çok altında yollanır (sınır ≤ 10 Hz).
- **Taşıyıcı.** `done` olayından sonra bugün kanal yok. İki seçenek var:
  - A) `State::Done` ilk taramanın emit kapanışını saklar.
  - B) Yeni bir `attach_scan(on_event: Channel<ScanEvent>) -> Option<ScanSummary>` komutu güncel `Done` taramaya yeni bir kanal bağlar ve öncekini değiştirir.
  - **Öneri: B.** Webview yenilenince eski JS kanalı zaten ölüdür; B hem canlı güncellemeyi hem de HMR sonrası yeniden bağlanmayı tek komutla çözer.
    - Tarama akışı değişmez: UI `done` alınca `attach_scan` çağırır.
    - Uygulama açılırken de çağırır: `Some` dönerse "Bitti" ekranına geri gelir.
  - Global `app.emit` kullanılmaz. Kanallar taramaya bağlıdır ve `store.ts` içindeki nesil (generation) kontrolü eski olayları zaten eler.
  - Yetenek (capability) değişikliği gerekmez.
- **UI tepkisi** (`store.ts` ve ekranlar):
  - `last.summary.root` ve döküm (`top`) güncellenir.
  - Açık görünüm (tablo sayfası, harita veya halka) bir kez yeniden çekilir. Şart: `dirs` boş değil ya da `truncated`.
  - Açık klasör detach olduysa `unknownNode` döner. UI iz (trail) üzerinde ilk yaşayan atasına çıkar ve durum satırında bunu söyler.
  - Sepet için mevcut `refreshBasket` çağrılır. Kaybolan öğeler sepetten düşer.
  - Seçili öğe kaybolduysa seçim kalkar.
  - Kök için tam yeniden tarama gerekirse UI bugünkü "Taranıyor" akışına girer.
  - Metinler TR ve EN olarak `i18n.ts` dosyasına eklenir (resmî ton).

#### Disk önbelleği değer mi? — Şimdilik hayır
- Yalnız 2. durumu (süreç yeniden başlaması) çözer. 1. durum `attach_scan` ile bedavaya çözülüyor.
- Önbellek kendi başına bayattır. Doğru kullanmak için kapanıştan beri olan değişiklikleri bilmek gerekir.
  - Bu, FSEvents olay kimliğiyle geriye oynatma (`sinceWhen`) ister.
  - `notify` bunu sunmuyor gibi görünüyor (açık soru). Bu da ikinci bir bağımlılık (ör. `fsevent-stream`) demek.
  - Geriye oynatma olmadan önbellek ya yanlış gösterir ya da yine tam tarama ister.
- Milyonlarca düğümlük bir ağaç diskte onlarca, yüzlerce MB tutar. Sürüm/biçim yönetimi ister.
- Kullanıcının bütün dosya listesini diske yazmak gizlilik açısından da hassas.
- **Ne zaman değer**: v1.x "Anlık görüntü ve fark" (`docs/PLAN.md` §2.2) ağacı zaten serileştirecek. Önbellek o işle birlikte ve FSEvents geriye oynatmasıyla ele alınmalı. "Not Now" listesine eklenir.

#### İş bölümü (her biri ayrı PR)
1. **`attach_scan` + HMR sonrası yeniden bağlanma.** — UYGULANDI (`feat/attach-scan`, onaylandı: "Evet, PR 1").
   - Bağımlılık yok. Küçük bir IPC değişikliği var.
   - "Dev'de her seferinde Tara" sıkıntısının yarısını hemen çözer.
   - Bu PR'da komut kanal almıyor: `attach_scan() -> Option<ScanSummary>`. Kanal parametresi `ScanEvent::Changed` ile birlikte (4. PR) eklenecek.
   - Sınır: tarama sürerken yapılan yeniden yükleme bağlanmaz; yalnız biten tarama geri açılır.
2. **Core: ağaç yamaları ve birleştirici.** — UYGULANDI (`feat/live-core-patches`, onay: "tamam devam").
   - `Tree::lookup`, yerinde güncelleme ve ekleme, alt ağaç aşılama, `refresh.rs`, `scan` için `root_dev` seçeneği.
   - Bağımlılık ve IPC değişikliği yok.
   - Yeni `pub` API: `Tree::{lookup, update_entry, add_child, graft, graft_children}`, `refresh::{Coalescer, Change, Depth, RefreshPlan, request, read, apply, refresh_dir, Outcome, Applied}`, `ScanOptions::root_dev`, `ScanResult::root_dev`.
   - Kökün altındaki yer tutucu (iCloud) ya da başka birim klasörleri yenilemede de okunmaz; tarayıcıyla aynı kural.
   - **PR 4 için sözleşme notları:**
     - Bayat okuma: `read` listesi uygulamanın çöpe taşımasından önce alındıysa `apply` o öğeyi geri ekler (sonraki olayda düzelir). Session, kendi çöpe taşımalarının çevresinde yenilemeyi sıralamalı ya da isteği yeniden toplamalı.
     - Bilinmeyen yol: `request` ağaçta olmayan yol için `None` döner. Oluşma olayı da kaybolmuş bir klasördeki `Change::Subtree` böylece hiç eklenmez. Session, bilinen ilk ataya çıkmalı (ya da birleştirici ebeveyni de sığ işaretlemeli).
3. **Platform: `watch.rs` (`notify`, yalnız macOS).** — UYGULANDI (`feat/live-watch`, onay: "Evet, PR 3").
   - `notify = "8.2.2"` yalnız macOS hedefinde; `tempfile` geliştirme bağımlılığı (çekirdekteki sürüm).
   - Yeni `pub` API: `watch::{watch, Watcher, FsChange, ChangeKind, WatchError}`. `Watcher` düşünce akış durur.
   - Olay kaybı ve akış hataları `Subtree`; okuma, sahip, izin, xattr ve Finder bilgisi değişiklikleri atılır.
   - FSEvents çözülmüş yolu bildirir (`/private/var/...`); yollar çağıranın kök yazımına geri çevrilir.
   - Ölçüldü: `/` izlenirken firmlink yolları `/Users/...` olarak geliyor; ek eşleme gerekmiyor.
   - **Karar bekleyen risk:** `notify` 8.2.2 FSEvents geri çağırmasında bilinmeyen bayrakta ya da UTF-8 olmayan yolda panic eder; `panic = "abort"` ile uygulama kapanır.
     - Bayraklar: macOS 27.2 SDK'daki 24 bayrağın hepsini tanıyor (bugün tetiklenmez).
     - Yollar: APFS/HFS+ adları UTF-8; ağ birimleri (NFS/SMB) olmayabilir.
     - `notify` 9.0.0-rc ikisini de düzeltiyor. Seçenek: 8.2.2'de kal ya da 9 rc'ye geç.
   - Ölçüldü: kök `/System/Volumes/Data` yazılırsa hiç olay gelmez (bu birim seçicide gizli).
   - Açık: Tam Disk Erişimi olmadan korumalı klasörlerden olay gelip gelmediği `tauri dev` altında ölçülemez (izin Terminal'e ait); paketli uygulamayla PR 4'te bakılacak.
4. **Session + `ScanEvent::Changed` + UI tepkisi.** ✅ 2026-11-17 (#35 merged) (`feat/live-session`).
   - **İzleyici ne zaman başlar:** tarama başlarken, `scan()` çağrısından önce. Tarama sırasında gelen değişiklikler birleştiricide birikir ve `Done` sonrasında uygulanır. Böylece uzun bir taramada erken taranan klasörlerdeki değişiklikler kaçmaz.
   - **Canlı iş parçacığı (`cleany-live-N`):** taramayla aynı ömürde.
     - Kanaldan `FsChange` alır ve `Coalescer`'a iter. `ready` olunca planı alır.
     - Her klasör için sırayla `request` (kilit altında), `read` (kilitsiz) ve `apply` (kilit altında, tarama hâlâ güncelse).
     - Durum `Done` değilse (tarama sürüyor) bekler. Tarama değiştirilmiş ya da iptal edilmişse çıkar.
   - **Yaşam süresi:** `Watcher` taramanın durumunda tutulur. Yeni tarama ya da iptal durumu değiştirince `Watcher` düşer, kanal kapanır ve iş parçacığı biter.
   - **Test için enjeksiyon:** `Sessions` bir izleme fabrikası alır. Varsayılanı `cleany_platform::watch`; testler sahte bir kaynak verir, gerçek FSEvents gerekmez ve 3 OS'ta deterministiktir. macOS dışında `Unsupported` döner; tarama bugünkü gibi anlık görüntü kalır.
   - **Uygulamanın kendi çöpe taşıması (bayat okuma):** `Finished` içinde bir `edits` sayacı tutulur ve `trash` bunu artırır. `apply` öncesinde sayaç `request` anındakinden farklıysa güncelleme uygulanmaz; klasör birleştiriciye geri konur ve sonraki turda yeniden okunur. Böylece çöpe taşınan öğe ekranda bir anlığına geri gelmez.
   - **Ağaçta olmayan yol:** `request` `None` dönerse ve yol bir klasörse, bilinen ilk ataya çıkılır ve o ata sığ yenilenir (yeni klasörler aşılanır).
   - **Tam yeniden tarama gereken durumlar:** `RefreshPlan::Full`, `Outcome::RootGone` ya da ölü düğüm sayısının canlıyı geçmesi. Bu durumlarda arka planda kendiliğinden yeniden taranmaz; yeni bir `ScanEvent::Outdated` olayı gönderilir ve izleme durur. Gerekçe: `/` taramasını sessizce yeniden başlatmak pahalıdır ve kullanıcı o sırada tabloda olabilir.
   - **Kanal:** `start_scan`'ın kanalı `Done` sonrasında da tutulur; ilk canlı olaylar bu kanaldan gider. `attach_scan(onEvent)` yeni bir kanal alır ve eskisinin yerine geçer (webview yenilendiğinde). Böylece `done` ile ilk bağlanma arasında olay kaybolmaz.
   - **Olaylar (IPC, onay gerekir):**
     - `{ event: "changed", revision, root }`: her uygulanan turda bir kez gönderilir (en çok 1 Hz civarı; birleştirme penceresi 1–5 sn). Ağaçta değişiklik yoksa gönderilmez.
     - `{ event: "outdated" }`: sonuçlar artık güvenilir değil, yeniden tarama gerekiyor.
     - Plandaki `dirs` ve `truncated` alanları çıkarıldı. Tablo, harita ya da halka bir klasörün tüm alt ağacının boyutunu gösterdiği için, derindeki her değişiklik açık görünümü etkiler. UI her `changed` olayında açık görünümü bir kez yeniden çeker; liste kullanılmayacağı için gönderilmez.
   - **UI tepkisi:**
     - `store.ts`: `changed` gelince `last.summary.root` güncellenir, döküm (`top`) yeniden yüklenir, sepet `refreshBasket` ile güncellenir ve `revision` artırılır.
     - Tablo, harita, halka ve döküm klasörleri `revision` değişince açık düğümü yeniden çeker. Bu sırada eski veri ekranda kalır; boş durum ya da yükleme göstergesi çıkmaz.
     - Açık klasör silindiyse (`unknownNode`), iz üzerindeki ilk yaşayan ataya çıkılır ve durum satırında "Açık klasör diskten kaldırıldı" bildirilir.
     - Seçili öğe kaybolduysa seçim kalkar.
     - `outdated` gelince döküm ve tablonun üstünde bir not gösterilir: "Tarama sonrasında diskte çok sayıda değişiklik oldu. Güncel sonuç için yeniden tarayın." ve "Yeniden tara" düğmesi. Eski sonuç görünür kalır.
     - Metinler TR ve EN olarak `i18n.ts` dosyasına eklenir (resmî ton).
   - **Kapsam dışı:** pencere gizliyken izlemeyi duraklatmak, aç/kapa ayarı, ağacı sıkıştırma.
   - **Testler:**
     - `session::changes_patch_done_tree_and_emit_changed`
     - `session::changes_buffered_during_scan_apply_after_done`
     - `session::changes_for_a_replaced_scan_are_dropped`
     - `session::trash_during_refresh_is_not_undone`
     - `session::unknown_dir_refreshes_first_known_ancestor`
     - `session::root_gone_emits_outdated_and_stops`
     - `session::attach_replaces_the_event_channel`
     - JSON şekli ve `ipc.ts` anahtar testleri `changed` ve `outdated` örnekleriyle genişler.
     - Testlerde birleştiricinin 1 sn'lik sessiz penceresi beklenir; birkaç test birkaç saniye sürer.
   - **Dosyalar:**
     - `src-tauri/src/session.rs` (canlı döngü ayrı bir `src-tauri/src/live.rs` modülüne çıkabilir)
     - `src-tauri/src/lib.rs`
     - `ui/src/lib/ipc.ts`, `ui/src/store.ts`, `ui/src/lib/i18n.ts`
     - `ui/src/screens/{Browse,Treemap,Sunburst,Summary}.tsx`
     - `CODEBASE_MAP.md`, `tasks/decisions.md`
   - **Doğrulama:**
     - 5 cargo adımı ve UI typecheck/build.
     - `tauri dev` ile canlı deneme: bir klasör taranır, Terminal'den dosya eklenip silinir, toplam ve tablo 1–5 sn içinde güncellenir.
     - FDA sorusu için paketli uygulama (`tauri build`) ile korumalı bir klasörde deneme.
5. **ADR-0006** ("Canlı izleme: FSEvents, sığ yenileme, kimlik koruma, önbellek yok"), PR 2 ile birlikte `tasks/decisions.md` dosyasına. — EKLENDİ.

#### Onay gereken değişiklikler
- **Yeni bağımlılık:** `cleany-platform` içine `notify`, yalnız macOS hedefinde, varsayılan `macos_fsevent` özelliğiyle.
  - Lisans: CC0-1.0.
  - Kendi debouncer crate'i (`notify-debouncer-full`) alınmaz. Birleştirme `refresh.rs` içinde yazılır ve test edilir.
- **IPC sözleşmesi:**
  - `ScanEvent::Changed` varyantı.
  - Yeni `attach_scan` komutu (`src-tauri/src/lib.rs` ve `ui/src/lib/ipc.ts`).
- **Yeni `pub` API:**
  - core: `Tree::lookup`, `Tree::refresh_dir` (veya eşdeğeri), `Tree::graft`, `refresh::Coalescer`, `ScanOptions::root_dev`.
  - platform: `watch::{Watcher, FsChange}`.
- **Yol haritası:** `docs/PLAN.md` §2.3'teki canlı izlemeyi öne çekmek.

#### Testler (koddan önce; her biri bugünkü ağaçta başarısız)
- `tree::update_entry_moves_size_delta_to_every_ancestor`: yerinde boyut güncellemesi kökte doğru toplamı verir.
- `tree::graft_appends_subtree_and_keeps_existing_ids`: aşılama sonrası eski kimlikler aynı düğümü gösterir, toplamlar doğrudur.
- `tree::lookup_finds_node_by_path_and_refuses_detached`: yoldan düğüme arama yapılır, detach edilmiş düğüm bulunmaz.
- `refresh::shallow_refresh_updates_files_detaches_gone_and_scans_new_dirs`: `tempdir` ile.
- `refresh::shallow_refresh_is_idempotent_after_app_trash`: uygulamanın kendi çöpe taşıması no-op olur.
- `refresh::coalesce_drops_paths_under_a_subtree_rescan`
- `refresh::coalesce_escalates_many_siblings_to_common_ancestor`
- `refresh::coalesce_ignores_paths_outside_root`
- `scan::subtree_scan_respects_original_root_device`
- `session::changes_patch_done_tree_and_emit_changed`: olay kaynağı enjekte edilir (`trash` testlerindeki `mover` gibi). Gerçek FSEvents gerekmez ve 3 OS CI'da deterministiktir.
- `session::changes_buffered_during_scan_apply_after_done`
- `session::changes_for_a_replaced_scan_are_dropped`
- `session::attach_scan_returns_current_done_summary_and_none_otherwise`
- `session::dead_nodes_over_live_trigger_full_rescan`
- Mevcut `scan_event_json_shape` ve `ipc_ts_mentions_every_serialized_key` testleri `changed` örneğiyle genişler.
- `watch::reports_created_file` (yalnız macOS, `tempdir` ve zaman aşımıyla): CI'da kararsız çıkarsa `#[ignore]` ile elle çalıştırılır.

#### Dosyalar
- `crates/cleany-core/src/tree.rs`: `lookup`, yerinde güncelleme, ekleme ve aşılama.
- `crates/cleany-core/src/refresh.rs` (yeni): birleştirici ve sığ yenileme.
- `crates/cleany-core/src/scan.rs`, `crates/cleany-core/src/lib.rs`: `root_dev` seçeneği ve yeniden dışa aktarımlar.
- `crates/cleany-platform/src/watch.rs` (yeni), `crates/cleany-platform/src/lib.rs`, `crates/cleany-platform/Cargo.toml`: onay sonrası.
- `src-tauri/src/session.rs`: `Done` içinde izleyici, tampon, yama, `Changed`, `attach`.
- `src-tauri/src/lib.rs`: `attach_scan` komutu.
- `ui/src/lib/ipc.ts`, `ui/src/store.ts`, `ui/src/screens/*`, `ui/src/lib/i18n.ts`: yeniden bağlanma ve `changed` tepkisi.
- `tasks/decisions.md` (ADR-0006), `CODEBASE_MAP.md` (bir satır), `docs/PLAN.md` (§2.3 notu).

#### Açık sorular
- **`notify` sürümü.** — Yanıtlandı: kararlı 8.2.2.
- **Geriye oynatma.** — Yanıtlandı: `notify` 8 `sinceWhen` açmıyor (akış hep "şimdi"den başlar). Önbellek gelirse ayrı crate gerekir.
- **`/` taramasında yollar.** — Yanıtlandı (ölçüldü): firmlink yoluyla (`/Users/...`).
  - Yol eşlemesi bunu bilmeli.
  - "Not Now"daki "Macintosh HD toplamı diski aşıyor" hatasıyla bağlantılı olabilir.
- **Tam Disk Erişimi (FDA).** FDA yokken korumalı klasörlerden olay geliyor mu? Gelmiyorsa o klasörler sessizce eski kalır.
- **Gizli pencere.** Pencere gizliyken izleme duraklasın mı?
- **Ayarlar.** Canlı güncelleme için bir aç/kapa ayarı gerekir mi? Öneri: ilk sürümde hep açık, ayar yok.

#### Riskler
- Sürekli değişen klasörler CPU ve pil tüketebilir. Birleştirme penceresi ve tam taramaya düşme eşiği bunu sınırlar; ölçülmeli.
- Kimlik korunsa da yeniden aşılanan alt ağaçlarda UI'ın tuttuğu eski kimlikler `unknownNode` olur. UI'ın ata'ya çıkma yolu sağlam olmalı.
- Hardlink toplamları yama sonrası yaklaşık kalır.
- Kilit altında süren uzun bir yama UI sorgularını bekletir. Yalnız ağaç güncellemesi kilit altında olmalı.

#### Doğrulama
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets`, `cargo test`, `cargo build --release`.
- `pnpm --dir ui typecheck && pnpm --dir ui lint && pnpm --dir ui build`.
- `tauri dev` içinde elle deneme:
  - `~/Downloads` tara, Finder'da dosya ekle, sil ve büyüt; birkaç saniyede görünmeli.
  - Bir klasörü komple sil; açık görünüm atasına çıkmalı.
  - ⌘R / HMR sonrası tarama yerinde olmalı.
  - Yoğun yazan bir klasörde (`cargo build` sırasında `target/`) CPU'yu izle.

### M4 — Korumalı yollar listesi + silme logu (Notion #32) ✅ 2026-11-16 (#28 merged)

**Durum (2026-11-16)**: #28 ile merge edildi. Uygulama içinden çöpe taşıma ve log dosyası elle denenmedi.

**Hedef**:
- İşletim sisteminin kendi klasörleri hiçbir zaman çöpe taşınamaz. Örnekler: `/System`, `/usr`, `C:\Windows`, `C:\Program Files`, ev dizininin kökü.
- Cleany'nin yaptığı her silme girişimi yerel bir log dosyasına yazılır: ne silindi, ne zaman, nereye ve sonucu ne oldu.
- Notion kartı boş; kabul ölçütü docs/PLAN.md:128 ve :130.

**Bağlam**:
- docs/PLAN.md:128 (korumalı yollar), :130 (silme logu), :185 ("Yanlış silme → … korumalı yollar, onay, log").
- #30'da gelen asgari koruma:
  - `cleany-platform::trash::is_protected_with` şunları korur:
    - taramanın kökü;
    - dosya sistemi kökü;
    - ev dizini ve atası;
    - bağlama noktaları.
  - `session::plan_trash` kökü ve bulut yer tutucularını korur.
  - `actions::move_one` sırası şöyledir: önce varlık kontrolü, sonra `is_protected`, sonra tür kontrolü, en son `move_to_trash`.
- Dal: `feat/m4-protected-log`.

#### Tasarım kararları — korumalı yollar
- **Liste `cleany-platform::trash` içinde, işletim sistemine göre (`cfg`) tanımlanır.** Her girdi üç ilişkiden birini taşır:
  - **Kendisi** (`Itself`): Yalnız klasörün kendisi korunur. İçindekiler serbesttir.
  - **Altı** (`Subtree`): Klasörün kendisi ve altındaki her şey korunur.
  - **Üstü** (her girdi için geçerli): Korumalı bir girdiyi içeren her klasör de korunur. Örneğin `/private`'ı taşımak `/private/etc`'yi de götürürdü.

  Önerilen liste:

  | OS | Altı (tamamı kilitli) | Yalnız kendisi |
  |---|---|---|
  | macOS | `/System`, `/bin`, `/sbin`, `/usr` (istisna: `/usr/local` altı serbest), `/private/etc`, `/private/var/db`, `/Library/Apple` | `/Applications`, `/Library`, `/Users`, `/Volumes`, `/private`, `/private/var`, `/opt`, `/usr/local`; ev altında `Library`, `Desktop`, `Documents`, `Downloads`, `Movies`, `Music`, `Pictures` |
  | Linux | `/bin`, `/sbin`, `/lib`, `/lib32`, `/lib64`, `/usr` (istisna: `/usr/local` altı), `/etc`, `/boot`, `/proc`, `/sys`, `/dev`, `/run` | `/home`, `/root`, `/var`, `/opt`, `/srv`, `/mnt`, `/media`, `/tmp`, `/usr/local`; ev altında `.config`, `.local`, XDG klasörleri |
  | Windows | `%SystemRoot%` (`C:\Windows`), `%ProgramFiles%`, `%ProgramFiles(x86)%` | `%ProgramData%`, `C:\Users`, ev altında `AppData`, `Desktop`, `Documents`, `Downloads` |

  - `/Volumes`, `C:\Users` ve `/Applications` "altı" olamaz: dış diskleri, uygulamaları ve ev dizinini kilitlerdi.
  - Windows'ta sistem sürücüsü sabit `C:` yazılmaz. Önce ortam değişkenleri okunur (`SystemRoot`, `ProgramFiles`…). Değişken yoksa `SystemDrive` kullanılır, o da yoksa `C:` varsayılır.
- **Eşleştirme kimlikle yapılır, metinle değil.**
  - Her girdi bir kez `read_meta` ile `(dev, file_id)` çiftine çevrilir; var olmayan girdiler atlanır.
  - Öğe için kendisi ve atalarının kimlikleri yürünerek karşılaştırılır.
  - Bu yöntemle büyük/küçük harf farkı (APFS, NTFS), Windows'taki `\\?\` öneki ve `/etc → /private/etc` gibi sembolik bağlantı yazımları sorun olmaktan çıkar.
  - Kimliği okunamayan yerlerde (Windows'ta `file_id` 0 olabilir) `canonicalize` ile yol karşılaştırmasına düşülür. Bu, var olan `same_entry` deseniyle aynıdır.
- **Kontrol sırası:** Liste kontrolü, `is_protected_with` içindeki "sembolik bağlantı → `false`" erken dönüşünden **önce** yapılır.
  - Öğenin kendisi bir girdiyse ya da bir girdinin altındaysa, `symlink_metadata` ile bağlantının kendisinin kimliği kontrol edilir.
  - Böylece listede bulunan, ama kendisi sembolik bağlantı olan bir yol (merged-usr Linux'ta `/bin`) yine korunur.
  - Bağlantının gösterdiği yer takip edilmez. `~/x → /usr` gibi bir bağlantı taşınabilir; çöpe yalnız bağlantı gider.
- **Test edilebilirlik:** Liste, `home` ve `mounts` gibi parametre olarak verilir (`is_protected_with(path, root, home, mounts, list)`). Testler geçici klasörlerle sahte bir liste kurar. Gerçek liste `is_protected` içinde bir kez kurulur.
- `TrashOutcome::Protected` doc yorumu ve UI metni güncellenir. Örnek TR metni: "… sistem klasörleri, uygulamalar klasörü, diskler, ana klasör ve üstü …". EN metni de aynı biçimde güncellenir.

#### Tasarım kararları — silme logu
- **Yer:** `app.path().app_log_dir()/deletions.jsonl`. macOS'ta bu `~/Library/Logs/com.tansuasici.cleany/deletions.jsonl` olur.
  - Yol Rust tarafında `AppHandle` ile çözülür.
  - Capability, `tauri.conf.json` ya da eklenti değişikliği gerekmez. Bu, uygulama sırasında doğrulanacak.
- **Biçim:** JSON Lines; her öğe için bir satır:
  ```json
  {"time":"2026-11-16T14:03:22Z","action":"trash","path":"/Users/x/a","kind":"dir","outcome":"trashed","to":"osTrash","logical":123,"allocated":4096,"message":null}
  ```
  - `action`: bu PR'da `trash`. #31 aynı biçimi `delete` değeriyle kullanır.
  - `to` ("nereye"): `osTrash`. İşletim sistemi çöpteki tam konumu döndürmediği için hedef konum değil, hedef türü yazılır. #31'de bu değer `permanent` olur.
  - Her sonuç yazılır: `trashed`, `gone`, `protected`, `failed`. Engellenen girişimler de iz bırakır.
- **Zaman damgası bağımlılıksız (önerilen):** `SystemTime` kullanılır ve UTC RFC 3339'a ~20 satırlık bir fonksiyonla çevrilir (Howard Hinnant'ın `civil_from_days` algoritması). Alternatif `time` crate'idir; bu yeni bağımlılık olur ve onay ister.
- **Kayıtların üretildiği yer:**
  - `Sessions::trash` raporla birlikte iç kayıtlar da döndürür: `(TrashReport, Vec<Deletion { path, kind, freed, outcome }>)`.
  - Kök ve yer tutucu öğeleri `plan_trash`'te `Protected` olur ve `mover`'a hiç ulaşmaz. Kayıtlar `Planned`'dan üretildiği için bu öğeler de loglanır.
  - `TrashItem` ve `TrashReport` değişmez. **IPC değişikliği yoktur.**
  - `actions::trash` kayıtları `deletion_log::append` ile yazar.
- **Yazma hatası:** Taşıma zaten gerçekleşmiştir, bu yüzden hata olarak dönmez ve panik oluşturmaz. `eprintln!("cleany: deletion log: …")` ile bildirilir. UI'da gösterilmez; gösterilmesi IPC değişikliği gerektirirdi.
- **Döndürme:** Dosya 5 MB'ı aşarsa `deletions.1.jsonl` adını alır. Tek yedek tutulur, eski yedeğin üzerine yazılır.
- **"Logu göster" düğmesi bu PR'da yok (önerilen).** Yeni IPC ve UI gerektirir; Not Now'a yazılır.

#### Onay gerekenler
- Yeni bağımlılık yok; `Cargo.toml` değişmez.
- IPC değişmez; `ipc.ts` değişmez.
- `tauri.conf.json`, capability ve CI dosyalarına dokunulmaz.
- Genel API'de iki değişiklik var:
  - `cleany-platform::trash::is_protected` imzası aynı kalır, ama davranışı genişler: liste eklenir.
  - `Sessions::trash` dönüş tipi `(TrashReport, Vec<Deletion>)` olur. Bu tip uygulama crate'inin içindedir; IPC'ye yansımaz.

#### Testler (önce yazılır)
- **platform `trash::tests`**, sahte listeyle geçici klasörlerde:
  - "kendisi" girdisi korunur, çocuğu serbesttir;
  - "altı" girdisinin torunu korunur;
  - `/usr/local` benzeri istisnanın altı serbesttir;
  - bir girdinin atası korunur;
  - listede olan bir sembolik bağlantı korunur;
  - korumalı klasörü gösteren bağlantı serbesttir;
  - macOS'ta büyük/küçük harf farklı yazım korunur.
- **platform, gerçek liste:**
  - macOS'ta `/System/Library`, `/usr/bin`, `/Applications`, `/etc` korunur;
  - Linux'ta `/usr/bin` ve `/etc` korunur;
  - Windows'ta `%SystemRoot%\System32` korunur.
- **`deletion_log::tests`:**
  - RFC 3339 dönüşümü: 0 → `1970-01-01T00:00:00Z`, artık gün `2024-02-29` ve yıl sonu;
  - iki çağrıda satırlar eklenir;
  - her satır `serde_json` ile geri okunur;
  - 5 MB sınırında döndürme yapılır;
  - yazılamayan klasörde `Err` döner, panik olmaz.
- **`session::tests`:** kök, yer tutucu, `trashed` ve `failed` öğelerinin hepsi kayıt üretir; yollar ve boyutlar doğrudur.

#### Dosyalar
- `crates/cleany-platform/src/trash.rs`: liste ve eşleştirme. Büyürse `protected.rs` adlı bir alt modüle ayrılır.
- `src-tauri/src/deletion_log.rs` (yeni), `actions.rs`, `session.rs`, `lib.rs` (`trash_nodes` `AppHandle` alır).
- `ui/src/lib/i18n.ts`: `trash.protected` metni (TR ve EN).
- `CODEBASE_MAP.md`.

#### Doğrulama
- Standart sıra uygulanır: `fmt --check`, `clippy --all-targets -D warnings`, `check`, `test`, `build --release`.
- **Çapraz hedef clippy** de çalıştırılır: `--target x86_64-pc-windows-msvc` ve `x86_64-unknown-linux-gnu`. `cfg` listeleri #30'daki Windows kırılmasıyla aynı riski taşır.
- Uygulamada gerçek deneme:
  - Önce SIP korumalı bir hedef (`/usr` ya da `/System` altındaki bir öğe) taranır ve çöpe taşınmaya çalışılır. Bir hata olsa bile SIP silmeyi engeller.
    - Beklenen sonuç `protected` olmalı, `failed` değil.
    - Log'da `"outcome":"protected"` satırı bulunmalı.
  - Ardından `~/cleany-trash-test` içindeki bir dosya taşınır.
    - Sonuç `trashed` olmalı ve log'da ilgili satır bulunmalı.

#### Açık sorular (önerilen varsayılanlar yukarıda)
1. Liste kapsamı tablodaki gibi mi? Özellikle ev altındaki standart klasörlerin (`Documents`, `Downloads`…) **kendileri**nin korunması.
2. Zaman damgası bağımlılıksız mı yazılsın, yoksa `time` crate'i mi kullanılsın?
3. "Logu göster" düğmesi sonraya mı bırakılsın?

#### Riskler
- Liste fazla geniş olursa kullanıcıyı engeller, fazla dar olursa korumaz. `/Library` ve `~/Library` yalnız kendileri korunduğu için içlerindeki önbellekler temizlenebilir kalır.
- Kimlik okuma maliyeti, girdi sayısı (~25) kadar `stat` çağrısıdır ve her taşıma isteğinde bir kez yapılır. Sepetteki (#33) çok öğeli isteklerde de liste bir kez kurulur.


### M4 — Çöpe taşı (varsayılan) + sonuç raporu (Notion #30) ✅ 2026-11-16 (#26 merged)

**Hedef**: Browse'da seçili öğe çöpe taşınabilsin: macOS'ta Çöp Sepeti, Windows'ta Geri Dönüşüm Kutusu, Linux'ta freedesktop çöpü. Taşımadan önce bir onay penceresi açılır. İşlem bitince bir sonuç raporu gösterilir: ne taşındı, ne kadar alan boşaldı, ne başarısız oldu. Ağaç ve boyutlar yeniden tarama gerekmeden güncellenir. Notion kartı boş; kabul ölçütü bu satır.

**Bağlam**:
- docs/PLAN.md:44, :61 (silme tek tıkla olmaz), :110 (`trash(nodes) -> Report`), :129 (`trash` crate adayı), :185.
- Komşu kartlar:
  - #31 kalıcı silme;
  - #32 korumalı yollar listesi ve silme logu;
  - #33 seçim sepeti;
  - #34 capability daraltma.
- Bu PR tek öğeyle çalışır, ama IPC baştan çoklu düğüm alır; sepet (#33) aynı komutu kullanacak.
- #29'un `(scan, node)` deseni ve `ActionBar` yeniden kullanılır. Dal `feat/m4-trash`.

#### Tasarım kararları
- **Bağımlılık: `trash` crate 5.3 (onay gerekir, önerilen).**
  - Platform kodu:
    - macOS'ta varsayılan yöntem Finder üzerinden çalışır; bu yöntem "Geri Koy"u destekler.
    - Windows'ta `IFileOperation` kullanır.
    - Linux'ta freedesktop çöp spesifikasyonunu uygular; bağlı disklerde `.Trash-uid` desteği vardır.
  - Yalnız `cleany-platform`'a eklenir.
  - Doğrulanacak: macOS'ta varsayılan yöntem Finder/AppleScript ise paketlenmiş uygulama `NSAppleEventsUsageDescription` ister. Bu, `tauri.conf.json` / Info.plist değişikliğidir (bundle yapılandırması, onay).
  - Alternatif olarak komut satırı kullanılabilir: `osascript` + Finder, PowerShell `Microsoft.VisualBasic.FileIO`, `gio trash`. Bu yolda bağımlılık yoktur, ama Windows'ta yavaştır, Linux'ta `gio` gerekir ve hata ayrıntıları zayıftır.
- **Genel API değişiklikleri (onay):** `cleany-core`'da `Tree::detach`, `Removed`, `NodeFlags::DETACHED`; `cleany-platform`'da `trash` modülü (`trash`, `is_protected`).
- **Yol yine UI'dan gelmez.** Yeni IPC (genel API, onay):
  - Komut: `trash_nodes(scan, nodes: number[]) -> TrashReport`.
  - Rapor: `{ items: [{ node, name, outcome }], freed: Size, trashed: number }`.
  - `outcome` değerleri: `trashed | gone | protected | failed{message}`.
  - Komut düzeyindeki hatalar (`staleScan`, `notReady`, `unknownNode`) mevcut `ActionError` ile döner.
- **İç içe seçim tekilleştirilir.** Listede bir düğüm ile onun atası birlikte varsa yalnız ata taşınır.
- **Asgari koruma, bu PR'da sabit kodlanır:**
  - taramanın kökü;
  - dosya sistemi kökleri (`/`, `C:\`);
  - ev dizini ve onun her atası (`/Users`, `/home`).
  - Bunlar `protected` döner ve dokunulmaz.
  - Diskteki öğenin türü taramadakinden farklıysa (klasör artık dosya, ya da tersi) `failed` döner ve taşınmaz.
  - Tam korumalı yollar listesi (`/System`, `/usr`, `C:\Windows`…) ve silme logu #32'de gelir (açık soru 2).
- **Ağacın güncellenmesi:**
  - `cleany-core`'a `Tree::detach(id) -> Option<Removed { size, files }>` eklenir.
  - Düğüm ebeveyninin kardeş zincirinden çıkarılır. Boyutu ve dosya sayısı tüm atalardan düşülür.
  - Arena'dan silinmez; id'ler kararlı kalır. Kopmuş düğüme `NodeFlags::DETACHED` (boş bit; 48 bayt testi korunur) konur.
  - `Tree::node_id` kopmuş düğümü ve kopmuş bir atanın altındaki düğümleri reddeder; bunun için ata zinciri yürünür. Böylece `path` ve `children` gerçekten `UnknownNode` döner.
  - Ardından `Finished.sorted` önbelleği temizlenir.
  - Sınır: sabit bağlantılar (hardlink) için yaklaşık bir değerdir (açık "Riskler").
- **Taşıma kilit dışında yapılır.** Kilit altında yollar toplanır, ardından kilit bırakılır. Taşıma bittikten sonra kilit yeniden alınır; tarama hâlâ aynıysa kopmuş düğümler uygulanır. Bu arada yeni bir tarama başlamışsa rapor yine döner, ama ağaca dokunulmaz.
- **Kopmuş düğüme yönelik istekler:**
  - `path` / `children` gibi istekler `UnknownNode` döner.
  - UI, gezilen klasör çöpe gittiyse en yakın canlı ataya döner.

#### UI kararları
- **Eylem:** `ActionBar`'a dördüncü düğme "Çöpe taşı" eklenir:
  - Kısayolu macOS'ta `⌘⌫`, diğerlerinde `Delete`'tir (açık soru 3).
  - Hedef, #29'daki gibi seçili öğedir. Hiçbir öğe seçili değilse ya da hedef taramanın köküyse düğme devre dışıdır.
- **Onay penceresi:**
  - Yerel `<dialog>` + `showModal()` kullanılır; odak tuzağı ve Esc tarayıcıdan gelir.
  - Başlık: "„X“ çöpe taşınsın mı?". Altında boyut ve klasörse dosya sayısı yazar.
  - Düğmeler "Vazgeç" ve "Çöpe taşı"dır; ilk odak "Vazgeç"tedir.
  - Kısayol yalnız pencereyi açar, hiçbir zaman doğrudan taşımaz.
  - Taşıma sürerken düğme "Taşınıyor…" olur ve devre dışı kalır.
- **Sonuç raporu:**
  - `ActionBar`'ın `role=status` satırına tek satırlık rapor yazılır: "„X“ çöpe taşındı · 1,2 GB boşaldı".
  - Başarısız ya da korumalı öğeler `role=alert` satırında nedeniyle birlikte gösterilir.
  - Çoklu rapor düzeni (liste) sepetle birlikte #33'te gelir.
- **Görünüm yenilenir:** Tablo, Harita ve Halka mevcut klasörü yeniden ister. Üst bilgideki toplam `done.summary` içinden geldiği için rapor yeni kök toplamlarını (`root: NodeEntry`) da taşır, UI özetteki kökü bununla değiştirir.
- **Hareket:** pencere için 150 ms ease-out opacity + `scale(0.98→1)` kullanılır, reduced-motion'da anında açılır. Rapor satırı canlandırılmaz.
- **Yeni token yok.** Pencere mevcut yüzey, kenarlık ve yarıçap token'larını kullanır. Tehlike rengi eklenmez; "Çöpe taşı" düğmesi birincil stildedir.

#### Testler (önce yazılır)
- `tree::tests::detach_*`:
  - atalardan boyut ve dosya sayısı düşer;
  - düğüm `children` listesinden çıkar;
  - iki kez detach edilen düğüm `None` döner;
  - kök detach edilemez.
- `session::tests::trash_*` (sahte çöp fonksiyonu enjekte edilir):
  - rapor doğru;
  - iç içe seçim tekilleştirilir;
  - kök/ev `protected` döner;
  - yok olmuş yol `gone` döner;
  - taşıma sırasında yeni tarama başlarsa ağaç değişmez.
- `trash::tests::*` (platform): `is_protected` için kök, ev ve sürücü kökü örnekleri.
- Gerçek çöpe taşıma: `temp_dir` altında bir dosya oluşturulur, taşınır, sonra yolun yok olduğu doğrulanır. Bu test `#[ignore]` olur, elle çalıştırılır; CI'da masaüstü çöpü olmayabilir.
- `actions::tests::ipc_ts_mentions_every_action_tag`: `'trash_nodes'` ve `outcome` etiketleriyle genişletilir.
- UI tarayıcı mock'uyla denenir:
  - onay, vazgeç, Esc;
  - rapor ve hata satırları;
  - görünümün yenilenmesi;
  - 375/768/1280, açık/koyu, klavye.
- Paketlenmiş `Cleany.app`'te geçici bir klasörle gerçek bir deneme yapılır; Finder'da "Geri Koy" çalışmalı.

#### Dokunulacak dosyalar
- `crates/cleany-core/src/tree.rs`: `Tree::detach`, `Removed`.
- `crates/cleany-platform/Cargo.toml` + `Cargo.lock`: `trash` (onay).
- `crates/cleany-platform/src/trash.rs` (yeni), `lib.rs`: `trash(path)`, `is_protected(path, scan_root)`.
- `src-tauri/tauri.conf.json` (gerekirse): `NSAppleEventsUsageDescription` (onay).
- `src-tauri/src/session.rs`: `Sessions::trash(scan, nodes, mover)`, `TrashReport`.
- `src-tauri/src/actions.rs`, `src-tauri/src/lib.rs`: `trash_nodes` komutu.
- `ui/src/lib/ipc.ts`: türler ve `trashNodes`.
- `ui/src/screens/BrowseActions.tsx`, `useBrowseActions.ts`: düğme, kısayol, onay penceresi, rapor.
- `ui/src/screens/Browse.tsx`: yenileme ve kopmuş klasörden ataya dönüş.
- `ui/src/lib/i18n.ts`: TR/EN metinler.
- `tasks/todo.md`: bu plan; #29 Done'a taşındı.

#### Açık sorular
1. Bağımlılık: `trash` crate 5.3 mü (önerilen), yoksa komut satırı mı?
2. Asgari koruma (tarama kökü, `/` ve sürücü kökleri, ev dizini) bu PR'da yeterli mi? Tam liste ve log #32'de gelecek. Alternatif: #32'yi önce yapmak.
3. Kısayol: macOS'ta `⌘⌫`, diğerlerinde `Delete` uygun mu? Finder ile aynıdır ve her zaman onay penceresi açar.
4. Onay her seferinde gösterilsin mi? Önerim: evet. "Bir daha sorma" seçeneği eklenmez (PLAN.md:61).

#### Riskler
- **macOS Finder yöntemi otomasyon izni isteyebilir.** İlk kullanımda "Cleany, Finder'ı denetlemek istiyor" sorusu çıkabilir. Reddedilirse `failed` döner ve mesajı gösterilir. Gerekirse `trash` crate'in `NsFileManager` yöntemine geçilir; o yöntemde soru çıkmaz, ama "Geri Koy" çalışmaz. Paketlenmiş uygulamada denenecek.
- **Hardlink ve klon (APFS) boyutları:** "boşaldı" değeri taramadaki ayrılmış boyuttur. Gerçek kazanç daha az olabilir; metinde "yaklaşık" demek gerekebilir.
- **Uzun süren taşıma** (çok dosyalı büyük klasör, Linux'ta farklı disk): komut `async` çalışır, UI'yı kilitlemez. İptal yoktur, ilerleme gösterilmez.
- **Windows ve Linux'ta elle deneme yok.** Yalnız CI derlemesi ve birim testleri var.

#### Uygulama durumu (2026-11-16, dal `feat/m4-trash`, commit yok)
Açık soruların hepsi önerildiği gibi onaylandı. Plandan sapmalar:
- **macOS yöntemi `NsFileManager`** (kullanıcı kararı). Otomasyon izni sorulmaz, `tauri.conf.json`'a dokunulmadı. Bedeli: Finder'da "Geri Koy" yok; diyalog metni yalnız "çöpten geri alınabilir" der.
- **Koruma genişledi** (kullanıcı kararı):
  - bağlama noktası olan ya da içeren klasörler `protected` döner;
  - indirilmemiş bulut yer tutucusu (iCloud/OneDrive) olan ya da içeren düğümler de `protected` döner.
  - Kimlik karşılaştırması `dev + file_id` ile yapılır, `/var` ile `/private/var` aynı sayılır. Sembolik bağlantı bağlantı olarak taşınır.
- `TrashReport.root` isteğe bağlıdır (`NodeEntry | null`). Taşıma sırasında tarama değiştiyse `null` döner ve ağaca dokunulmaz.
- `gone` dönen düğüm de ağaçtan koparılır.
- Taşıyıcı fonksiyon tarama kökünü de alır, `is_protected` için.
- **Diyalog animasyonsuz.** Planda 150 ms vardı; ama diyalog çoğunlukla kısayolla açılıyor, klavye eylemleri canlandırılmıyor.
- **Kısayol** bölümde `onKeyDownCapture` ile yakalanır, böylece görünümlerin Backspace (üst klasör) işleyicisi ⌘⌫'yu görmez. Diyalog açıkken eylem kısayolları çalışmaz.

Doğrulama:
- `fmt`, `clippy -D warnings`, `check`, `test` (app 36, platform 19 + 1 ignored) ve `build --release` temiz.
- `pnpm typecheck`, `lint` ve `build` temiz.
- Tarayıcı mock'unda denendi:
  - ⌘⌫ ile açılış, ilk odak Vazgeç'te;
  - Esc ile kapanış, odak ızgaraya dönüyor;
  - onay sırasında "Taşınıyor…";
  - durum satırı ve boyutların yenilenmesi (Movies 18,0 → 4,6 GB);
  - `failed` uyarı satırı; bu durumda ağaç değişmiyor;
  - 375 koyu ve 768 açık.
- Paketlenmiş `Cleany.app` (release) ile gerçek deneme, 2026-11-16, `~/cleany-trash-test` üzerinde:
  - ⌘⌫ diyaloğu açıyor, ilk odak Cancel'da; Esc kapatıyor.
  - Dosya (`deneme.txt`) ve klasör (`klasor`, "Folder · 4.1 KB · 1 file") çöpe gitti; liste, toplam ve durum satırı yenilendi ("Moved … · 4.1 KB freed").
  - Gezilen klasörün kendisi çöpe gidince ataya (`tansuasici`) dönüldü.
  - Üçü de Finder Çöp'ünde, `~/cleany-trash-test` diskte yok.
  - Not: arka plan otomasyonu `aria-haspopup="dialog"` butonuna basamıyor, enjekte tuşlar da web görünümüne ulaşmıyor; deneme ön planda yapıldı.
- **Kalan:** yok.

### M4 — Göster / aç / yolu kopyala (Notion #29) ✅ 2026-11-15 (#23 merged)

**Hedef**: Browse'un üç görünümünde (Tablo, Harita, Halka) seçili öğe için üç eylem: dosya yöneticisinde göster, varsayılan uygulamayla aç, tam yolu panoya kopyala. Bu eylemler düğmelerle ve klavyeyle yapılabilmeli. macOS, Windows ve Linux'ta çalışmalı. Notion kartı boş; kabul ölçütü bu satır.

**Bağlam**: docs/PLAN.md:44 (Aksiyonlar), :110 (`reveal(node)`, `open(node)`), :72 (`cleany-platform` = "çöp/reveal"). M4'ün ilk kartı; #33 (sepet) ve #30 (çöp) aynı "seçili öğe" bağlantısını kullanacak. Tek PR, dal `feat/m4-reveal-open-copy`.

#### Durum (2026-11-15)
Plan onaylandı; açık sorular: 1 `R`/`O`/`⌘C`, 2 evet, 3 `std::process::Command`, 4 bağlam menüsü yok. İki commit (simgeler, özellik), #23 ile merged.
- Plandan sapmalar: Rust tarafı `ActionError` ve komut gövdeleri `src-tauri/src/actions.rs`'e alındı (`lib.rs` yalnız kaydeder). Kancalar oxlint `only-export-components` nedeniyle `ui/src/screens/useBrowseActions.ts`'e ayrıldı. "Kopyalandı" için 150 ms solma eklenmedi (geri bildirim bir tuşa yanıt; tuş eylemleri canlandırılmaz).
- Doğrulama: fmt/clippy/test/release temiz. Tarayıcı mock'uyla üç görünümde ok tuşuyla seçim → R/O doğru düğüm id'sini çağırıyor, ⌘C yolu kopyalıyor, "artık yok" uyarısı çıkıyor; 375/768/1280, açık/koyu.
- Ayrı değişiklik (ayrı commit): macOS/Windows uygun simgeler (`assets/icon-macos*.svg`, `src-tauri/icons/`).

#### Tasarım kararları
- **Yol UI'dan gelmez.** Komutlar `(scan, node)` alır; yol Rust'ta ağaçtan çıkarılır (`Tree::path`). Böylece webview rastgele bir yolu açtıramaz. Bu, #34'teki yetki daraltma çizgisiyle de uyumlu.
- **Yeni bağımlılık yok (önerilen).** `cleany-platform`'a `shell` modülü eklenir. `access::open_settings` gibi `std::process::Command` kullanır:
  - macOS: göster `open -R <yol>`, aç `open <yol>`.
  - Windows: göster `explorer.exe /select,"<yol>"` (`raw_arg`, virgül/boşluk için), aç `explorer.exe "<yol>"`.
  - Linux: göster `dbus-send … org.freedesktop.FileManager1.ShowItems`, yoksa üst klasörde `xdg-open`; aç `xdg-open <yol>`.
  - Alternatif: `tauri-plugin-opener` (`reveal_item_in_dir`, `open_path`). Linux'ta daha denenmiş, ama yeni bağımlılık olduğu için onay ister. Yalnız Rust tarafından çağrılır, JS'e izin açılmaz.
- **Komut kurma saf fonksiyon.** `fn reveal_command(os, path) -> (program, args)` ve `fn open_command(os, path) -> (program, args)`, OS'tan bağımsız birim testiyle sınanır. Gerçek çalıştırma ince bir sarmalayıcıdır.
- **Tarama ile şimdi arasında yol kaybolmuş olabilir.** Çalıştırmadan önce `symlink_metadata` ile bakılır. Yol yoksa `ActionError::Gone` döner; UI "Bu öğe artık yok" der.
- **Yeni IPC (genel API değişikliği, onay):**
  - `reveal_node(scan, node) -> ()`
  - `open_node(scan, node) -> ()`
  - `get_path(scan, node) -> String`
  - Hata türü `ActionError { kind: notReady | unknownNode | gone | launch{message} }`, `ipc.ts` ile eşlenir; mevcut "etiket kayması" testi genişletilir.
- **Kopyalama UI'da.** Yol `get_path` ile alınır, `navigator.clipboard.writeText` ile yazılır (Tauri webview güvenli bağlam; kullanıcı hareketiyle tetiklenir). Clipboard eklentisi yok.
- **Aç ≠ Enter.** Enter ve → bugünkü gibi yalnız klasörün içine girer. "Aç" ancak düğmeyle ya da kendi kısayoluyla çalışır; böylece kazara bir dosya ya da uygulama başlatılmaz. Bir dosyayı açmak Finder'daki çift tıkla aynı şeydir; yürütülebilir dosyalar engellenmez (açık soru 2).

#### UI kararları
- **Eylem satırı:** görünümün altında, ipucu satırının yerinde (ipucu sağa kayar ya da altında kalır; 375'te alt alta). Üç ikincil düğme vardır:
  - "Finder'da göster" (Windows'ta "Gezgin'de göster", Linux'ta "Dosya yöneticisinde göster")
  - "Aç"
  - "Yolu kopyala"
- **Hedef:** seçili öğe. Hiçbir öğe seçili değilse, ya da seçim "diğerleri" kümesiyse, hedef o an gezilen klasördür. Düğme etiketleri hedefin adını göstermez; `aria-describedby` hedefi söyler.
- **Görünümlerden seçim yukarı taşınır:** Table, Treemap ve Sunburst'e `onSelect(target | null)` eklenir (`{ id, name, kind }`).
- **Kısayollar** (odak görünümdeyken): `R` göster, `O` aç, `⌘C` / `Ctrl+C` yolu kopyala. İpucu satırı bunları söyler (açık soru 1).
- **Geri bildirim:**
  - Kopyalayınca düğmenin yanında 2 sn "Kopyalandı" görünür (`aria-live="polite"`); tetikleyen düğme kayıp yaratmaz.
  - Hata satır içinde `role="alert"` ile gösterilir, renk şeridi yoktur.
  - Hareket yalnız opacity, 150 ms ease-out, reduced-motion'da anında.
- **Yeni token yok;** mevcut ikincil düğme stili kullanılır.

#### Testler (önce yazılır)
- `shell::tests::reveal_command_*` (macos/windows/linux): program ve argümanlar beklenen biçimde; Windows'ta virgüllü ve boşluklu yol bozulmaz.
- `shell::tests::open_command_*` (her OS).
- `shell::tests::linux_reveal_falls_back_to_parent`: dbus yoksa üst klasör açılır.
- `session::tests::path_of_node_is_root_joined`: kök ve derin düğümün yolu doğru.
- `session::tests::path_of_unknown_node_is_error` / `path_before_done_is_not_ready`.
- `lib::tests::ipc_ts_mentions_every_action_tag`: `ActionError` etiketleri `ipc.ts`'de var.
- UI'da test koşucusu yok; tarayıcı mock'uyla elle doğrulanır:
  - üç görünümde seç → göster, aç ve kopyala çalışır;
  - "diğerleri" seçilince hedef klasördür;
  - hata ve "artık yok" durumları görünür;
  - 375/768/1280, açık ve koyu, klavye.

#### Dokunulacak dosyalar
- `crates/cleany-platform/src/shell.rs` (yeni), `crates/cleany-platform/src/lib.rs`: `reveal`, `open`, komut kurucular.
- `src-tauri/src/session.rs`: `Sessions::path(scan, node)`.
- `src-tauri/src/lib.rs`: üç komut, `ActionError`, kayıt.
- `ui/src/lib/ipc.ts`: türler ve `revealNode` / `openNode` / `getPath`.
- `ui/src/screens/Browse.tsx`: eylem satırı, hedef durumu.
- `ui/src/screens/Treemap.tsx`, `ui/src/screens/Sunburst.tsx`, (Table `Browse.tsx` içinde): `onSelect` ve kısayollar.
- `ui/src/lib/i18n.ts`: TR/EN metinler (OS'a göre "göster" etiketi `app_info.os`'tan).
- `tasks/todo.md`: bu plan; biten M3 bölümleri Done'a taşınır.

#### Açık sorular
1. Kısayollar `R` / `O` / `⌘C` uygun mu? Alternatif: `⌘⇧R` göster (VS Code "Reveal in Finder" `⌥⌘R`), `⌘O` aç.
2. "Aç" yürütülebilir dosyalarda (`.app`, `.exe`, betikler) da çalışsın mı? Önerim: evet, Finder'daki gibi. Kazara açmayı Enter'dan ayırmak yeterli.
3. Bağımlılık: düz `std::process::Command` (önerilen) mi, yoksa `tauri-plugin-opener` mı?
4. Sağ tık bağlam menüsü bu PR'a girmesin (M4'te sepetle birlikte daha anlamlı), uygun mu?

#### Riskler
- **Linux'ta dosya yöneticisi çeşitliliği:** `FileManager1` olmayan masaüstünde öğe seçilemez, yalnız klasör açılır. Kabul edilir, elle Linux denemesi #27 ile birlikte yapılır.
- **Windows'ta `explorer /select` tırnaklama hatası:** `raw_arg` ve testle kapatılır. Windows'ta elle denenemiyor; CI derlemesi var.
- **WebKit `clipboard.writeText` kullanıcı hareketi ister:** `get_path` yanıtı beklenirken hareket bağlamı düşebilir. Önlem: seçim değişince yol önceden alınır, düğmede doğrudan yazılır. Olmazsa `tauri-plugin-clipboard-manager` gerekir (onay).

### M3 — Görselleştirme: sunburst (Notion #25) ✅ 2026-11-15 (#22 merged)

**Hedef**: Browse'da "Tablo / Harita" yanına üçüncü bir "Halka" görünümü. Klasör merkezde daire, alt klasörler dışa doğru iç içe halkalar (açı = boyut payı), canvas'ta çizilir. Hover bilgi verir, dilime tıklamak içine girer, merkeze tıklamak üst klasöre çıkar, klavyeyle tam kullanılır. Notion kartı boş; kabul ölçütü bu satır.

**Bağlam**: docs/PLAN.md:40 (sunburst alternatif görünüm), :115 (canvas, yerleşim d3-hierarchy ya da Rust), :171 (M3). Treemap (#26/#24) deseni izlenir: yerleşim Rust'ta, UI yalnızca çizer. Tek PR.

#### Tasarım kararları
- **Yerleşim Rust'ta**, `d3-hierarchy` eklenmez (treemap ile aynı gerekçe: bağımlılık yok, `cargo test` ile sınanır). Sunburst yerleşimi basit: her çocuk, ebeveyninin açı aralığını boyut payına göre böler.
- **Yeni modül** `cleany-core::sunburst` + **yeni IPC komutu** `get_sunburst(scan, node, depth, minAngle) -> SunburstSlice` *(korumalı: public API + IPC yüzeyi)*.
- **Açılar tur cinsinden** (0‥1, saat 12'den saat yönünde). Piksel boyutu gerekmez; yarıçapları UI seçer, pencere boyutu değişince yeniden istek atılmaz.
- **Çıktı düz liste**, ebeveyn çocuklarından önce: `Arc { id: number|null, parent, depth, start, end, allocated, files, name, kind, hasChildren, others }`. Treemap'teki `TreemapCell` alanlarıyla aynı, yalnız `x,y,w,h` yerine `start,end`.
- **Eşik**: açısı `minAngle` (tur, UI varsayılanı ~0.004 ≈ 1.4°) altında kalan çocuklar tek "Diğer N öğe" dilimine birleşir (`id: null`). Toplam açı korunur. Tek kalan küçük çocuk birleştirilmez (treemap kuralı).
- **Derinlik**: UI 3 halka ister; üst sınır mevcut `MAX_TREEMAP_DEPTH = 3` yerine ayrı `MAX_SUNBURST_DEPTH = 4`. `MAX_CELLS = 5_000` ortak.
- Boyut `allocated`; sıfır baytlık ve hardlink düğümleri girmez (treemap ile aynı filtre, ortak yardımcıya taşınmaz — iki küçük kopya, refactor yok).

#### UI kararları
- `ui/src/screens/Sunburst.tsx`, `Treemap.tsx`'in desenini kopyalar: `role="listbox"` panel + `aria-activedescendant`, sr-only seçenekler (ilk halka), aria-hidden canvas ve hover kartı, `role="alert"` hata + Retry, tek `role="status"` satırı, alt bilgi şeridi.
- Treemap’teki `useBox`, `useScheme`, `fit` yardımcıları ortak `ui/src/canvas.ts` dosyasına taşınır (Treemap.tsx’ten export edilince oxlint `only-export-components` uyarıyordu).
- **Renk**: treemap ile aynı yön (DESIGN_MAP onaylı): mürekkep düşük opaklıkta, seviye başına bir adım; yalnız en büyük ilk-halka dilimi accent. Dilimler arasında 1px kart rengi ayırıcı. Yeni token yok.
- **Merkez**: kart renginde daire, içinde klasör adı + boyutu. Tıklama / Backspace üst klasör.
- **Klavye**: ←/→ (ve ↑/↓) ilk halkada saat yönünde/tersine dolaşır, Home/End, Enter içine girer, Backspace üst, Esc döküm. Seçili dilim accent kontur.
- **Hover**: dilim + halka için açı/yarıçap isabet testi; kart treemap'tekiyle aynı.
- **Görünüm geçişi**: `View = 'table' | 'map' | 'rings'`, localStorage'da hatırlanır. Etiket TR "Halka" / EN "Rings", ipucu satırı `browse.ringsHint`.
- Animasyon yok (içine girme anında çizilir) — motion kuralı: sık eylem.
- Küçük ekran: daire `min(w, h)` ile sığar, 375px'te de okunur; dilim etiketi yazılmaz (yalnız hover/şerit), merkez etiketi yazılır.

#### Testler (önce yazılır)
- `cleany-core::sunburst`:
  - `children_split_parent_span_by_size` — açılar boyut payıyla orantılı, ardışık, boşluksuz.
  - `level_spans_add_up_to_parent` — her ebeveynin çocuk aralıkları ebeveyn aralığına eşit (ε içinde).
  - `small_children_merge_into_others` — eşik altı çocuklar tek "Diğer", `others` sayısı ve açısı doğru.
  - `single_small_child_is_not_merged`.
  - `depth_limit_is_respected` / `max_cells_is_respected`.
  - `zero_and_hardlink_nodes_are_skipped`.
  - `parents_precede_children`.
- `src-tauri/session.rs`: `get_sunburst_rejects_stale_scan_and_unknown_node`, `get_sunburst_clamps_depth_and_angle`, DTO serde şekli (`camelCase`, `id: null`).
- UI: test altyapısı yok → typecheck + lint + build, mock ile tarayıcıda 375/768/1280 × açık/koyu, klavye turu, hata/boş/yükleniyor durumları.

#### Dokunulacak dosyalar
- `crates/cleany-core/src/sunburst.rs` (yeni), `crates/cleany-core/src/lib.rs` (modül + re-export)
- `src-tauri/src/session.rs` (DTO + `Sessions::sunburst` + testler), `src-tauri/src/lib.rs` (komut kaydı)
- `ui/src/lib/ipc.ts` (tipler + `getSunburst`), `ui/src/screens/Sunburst.tsx` (yeni), `ui/src/canvas.ts` (yeni, ortak tuval yardımcıları), `ui/src/screens/Treemap.tsx` (yardımcıları canvas.ts’ten alır), `ui/src/screens/Browse.tsx` (üçüncü görünüm), `ui/src/lib/i18n.ts` (TR/EN metinler)
- `CODEBASE_MAP.md` gerekirse tek satır.

#### Açık sorular
- Görünüm adı: "Halka" / "Rings" uygun mu? (Alternatif: "Sunburst" iki dilde de.)

#### Riskler
- Çok sayıda ince dilimde WebKitGTK çizim süresi — #27 ölçecek; `MAX_CELLS` ve `minAngle` sınırlar.
- İsabet testi: açı sarması (0/1 sınırı) ve yarıçap halkaları; testte kenar durumları elle denenir.

### M3 — Görselleştirme: treemap (Notion #26 → #24) ✅ 2026-11-15 (#20, #21 merged)

**Hedef**: Bitmiş bir taramada bir klasör, canvas üzerinde squarified treemap olarak çizilir. Hover bilgi verir, bir bloğa tıklamak içine girer, breadcrumb ile geri dönülür. UI'a yalnızca çizilecek kadar düğüm gider.

**Bağlam**: docs/PLAN.md §3.4 (`get_treemap(node, depth, min_area)`), §3.5 (canvas, DOM ile çizilmez), §9 M3. CLAUDE.project.md: IPC'ye dilim gider, bütün ağaç asla. Sıra: önce #26 (Rust), sonra #24 (UI). İki ayrı PR.

#### PR bölümü
| PR | Kart | İçerik | Yeni bağımlılık | Onay gereken |
|---|---|---|---|---|
| 6 `feat(core): treemap layout and slicing` | #26 | `cleany-core::treemap`: squarified yerleşim + min alan eşiği + derinlik sınırı; `get_treemap` komutu; `ipc.ts` tipleri | yok | core public API'ye yeni modül, yeni IPC komutu |
| 7 `feat(ui): canvas treemap` | #24 | Canvas bileşeni, hover kartı, tıkla-içine-gir, breadcrumb, Summary/Browse'a "Harita" görünümü | yok (d3-hierarchy eklenmez; yerleşim Rust'ta) | yön/token: blok renkleri, görünümün yeri |

#### Tasarım kararları (PR 6)
- **Yerleşim Rust'ta yapılır**, `d3-hierarchy` eklenmez. Gerekçe: bağımlılık yok, deterministik ve `cargo test` ile sınanır, UI yalnızca dikdörtgen çizer. Eşik de piksel cinsinden olduğu için yerleşimle aynı yerde olmalı.
- **İmza**: `get_treemap(scan, node, width, height, depth, minArea) -> TreemapSlice`. `width`/`height` CSS pikseli (squarify en-boy oranına bağlı olduğu için şart). `minArea` px² (varsayılan UI'da ~ 24 px²). `depth` en fazla 3 ile sınırlanır.
- **Çıktı düz bir liste**: `TreemapSlice { node, cells: Vec<Cell> }`, `Cell { id: Option<u32>, parent: u32, depth: u8, x, y, w, h: f32, allocated, name, kind, hasChildren, others: u32 }`. Ebeveyn, çocuklarından önce gelir; UI sırayla çizer.
- **Eşik altı kalanlar birleştirilir**: bir ebeveynin `minArea` altındaki çocukları tek bir "Diğer N öğe" hücresinde toplanır (`id: null`, `others: N`). Toplam alan korunur, hiçbir bayt kaybolmaz.
- **Üst sınır**: dilim başına en fazla `MAX_CELLS = 5_000` hücre (WebKitGTK bütçesi, #27 ölçecek). Sınır aşılırsa derinlik kısaltılır, düğüm atılmaz.
- **İç boşluk**: her klasör hücresi çocuklarını 1 px içeri çekilmiş bir alana yerleştirir (UI çizgisiyle uyumlu). Klasör başlık şeridi yok; ad hover kartında ve yeterince büyük bloklarda gösterilir.
- **Boyut**: `allocated` (dökümle aynı). Sıfır baytlık ve hardlink düğümleri (bayt başka yerde sayılmış) yerleşime girmez.
- Mantık `cleany-core`'da, `session.rs` yalnızca kilit + DTO; `get_children` desenine uyar.

#### Testler (PR 6, önce yazılır)
- `squarify_preserves_total_area` — çocuk alanları ebeveyn alanının toplamına eşit (ε içinde).
- `squarify_cells_stay_inside_and_do_not_overlap` — her hücre ebeveyn içinde, ikili kesişim yok.
- `squarify_aspect_ratio_is_bounded` — eşit boyutlu 16 çocukta en kötü oran ≤ 2.
- `small_children_merge_into_others` — eşik altı çocuklar tek "Diğer" hücresi, `others` doğru, alanı toplamlarına eşit.
- `depth_limit_is_respected` / `max_cells_is_respected`.
- `zero_and_hardlink_nodes_are_skipped`.
- `empty_and_single_child_dirs` — boş klasör tek hücre, tek çocuk ebeveyni doldurur.
- `get_treemap_rejects_stale_scan_and_unknown_node` (session).
- `ipc_ts_mentions_every_serialized_key` yeni anahtarları kapsar.

#### PR 7 (#24) planı — onaylandı 2026-11-15 (`feat/m3-treemap-ui`)
- **Yön (onaylı)**: blok renkleri kind'a göre değil, derinliğe göre yarı saydam ink tonları (üst üste binince koyulaşır); en büyük blok accent ile tonlanır, gökkuşağı yok. Görünüm Browse ekranında Tablo / Harita anahtarıyla seçilir (seçim tarayıcıda hatırlanır).
- **Dosyalar**: `ui/src/screens/Treemap.tsx` (yeni: canvas, hover kartı, klavye, gizli liste), `ui/src/screens/Browse.tsx` (anahtar + görünüm seçimi), `ui/src/lib/i18n.ts` (TR/EN metinler). IPC, Rust, token değişikliği yok.
- **Çizim**: `devicePixelRatio` ölçekli canvas; `ResizeObserver` + 120 ms debounce ile `getTreemap(scan, node, w, h, 2, 24)`; eski yanıtlar düşer. Hücreler `Math.round` ile hizalanır, kardeşler arasında 1 px boşluk kalır. Ad + boyut yalnızca yeterince büyük 1. seviye bloklara yazılır. Renkler CSS değişkenlerinden okunur, tema değişince yeniden çizilir.
- **Etkileşim**: tıklama, imlecin altındaki 1. seviye klasöre girer. Seçili blok accent çerçeve alır. Hover kartı en derindeki hücreyi gösterir (ad, boyut, pay; "Diğer N öğe"). İçine girerken çocuğun dilimi önceden istenir, `browse(slice.node)` aynı dilimi yeniden kullanır (çift istek yok).
- **Klavye**: oklar 1. seviye bloklar arasında yöne göre gezer, Enter içine girer, Backspace üst klasör, Esc döküme döner (tablo ile aynı). Üst klasöre çıkınca gelinen blok seçili gelir.
- **Erişilebilirlik**: odaklanabilir `listbox`, 1. seviye bloklar görünmez `option` olarak (aria-activedescendant); canvas `aria-hidden`.
- **Durumlar**: yükleniyor (250 ms gecikmeli), hata + yeniden dene, boş klasör.
- **Doğrulama**: `pnpm --dir ui typecheck/lint/build`; 375/768/1280 × açık/koyu ekran görüntüsü; klavye; `/slop-audit`.

#### Açık sorular
- Varsayılan `minArea` ve `depth`: öneri 24 px² ve 2 seviye; #27 benchmark'ı sonrası ayarlanır.
- `/` çift sayım hatası treemap'te de görünür (System + Users şişkin). Ayrı iş, Not Now'da duruyor.

#### Riskler
- f32 yuvarlama ile 1 px'lik boşluk/taşma: testlerde ε kullanılır, UI'da çizerken `Math.round` hizalaması.
- Çok geniş klasörlerde (100k çocuk) sıralama maliyeti: `get_children`'daki sıralama önbelleğine benzer şekilde yalnız bir kez sıralanır; ölçülür.

### M2 — Uygulama kabuğu (Notion #18–#23) ✅ 2026-11-15 (#17 dahil merged)

**Hedef**: Uygulama açılınca birimler (volume) listelenir. Kullanıcı bir birimi ya da klasörü tarar, canlı ilerlemeyi görür ve taramayı iptal edebilir. Sonuç sanal bir ağaç tabloda boyuta göre sıralı gelir; tek tıkla klasörün içine girilir (ADR-0004), breadcrumb ile geri çıkılır. UI hiçbir zaman milyonlarca düğüm almaz, yalnızca sayfa sayfa çocuk listesi alır.

**Bağlam**: docs/PLAN.md §3.4 (IPC), §3.5 (frontend), §9 (M2 kabul ölçütleri). Notion #5 (`/design-brief` + `/direction`) hâlâ "Devam ediyor" ve `ui/DESIGN_MAP.md` boş şablon. `ui/CLAUDE.md`, DESIGN_MAP boşken UI yazılmamasını söylüyor. Bu yüzden backend PR'ları önce gelir, UI PR'ları yön onayına bağlıdır.

#### PR bölümü
| PR | Kartlar | İçerik | Yeni bağımlılık (onay gerekir) | Tasarıma bağlı mı |
|---|---|---|---|---|
| 1 `feat(app): scan session IPC` | #18, #20 (backend) | Tek oturumluk tarama kaydı, `start_scan` / `cancel_scan` / `get_children` / `get_ancestors`, ilerleme `tauri::ipc::Channel` üzerinden (core zaten ≤10 Hz), DTO'lar, TS tipleri | Elle TS tipleri (tauri-specta RC) + drift testi | Hayır |
| 2 `feat(platform): volume list` | #19 (backend) | `list_volumes() -> Vec<Volume { mount, name, total, available, removable }>`, `list_volumes` komutu | `sysinfo` (yalnızca `disk` özelliği) (Soru 2) | Hayır |
| 3 `feat(ui): shell, volumes, scan screen` | #19, #20 (UI), #23 | Uygulama kabuğu, birim listesi, ilerleme + iptal ekranı, TR/EN sözlük, açık/koyu tema tokenları | `zustand` | **Evet** |
| 4 `feat(ui): tree table` | #21 | Sanal liste, sütuna göre sıralama, tek tıkla içine girme, breadcrumb, klavye ile gezinme | `@tanstack/react-virtual` | **Evet** |
| 5 `feat(app): macOS Full Disk Access onboarding` | #22 | FDA durumunu tespit eden komut ve Sistem Ayarları'nın ilgili bölmesini açan komut, onboarding ekranı | yok (Rust tarafında sabit URL ile `open`) | **Evet** |

#### Tasarım kararları
- **DTO'lar `src-tauri` içinde kalır**; core'a serde eklenmez. `ScanIssue` bir `io::Error` taşıdığı için zaten DTO'ya çevrilmesi gerekir. Adlar yalnızca gösterim için `to_string_lossy` ile gönderilir.
- **Düğüm kimliği `(ScanId, düğüm indeksi)` çiftidir**, asla yol string'i değildir. Her `get_children` çağrısı `ScanId`'yi geçerli oturumla karşılaştırır; eşleşmezse tipli `StaleScan` hatası döner. Core'a tek bir public ekleme yapılır: `Tree::node_id(u32) -> Option<NodeId>` (aralık kontrollü). Breadcrumb için `Node::parent` zaten var.
- **Oturum durumu bir enum'dur**: `Idle | Scanning { id, cancel } | Done { id, result } | Cancelled { id } | Failed { id, error }`. `scan()` bloklayan bir çağrıdır, kendi thread'inde çalışır. Tarama sürerken yeni bir `start_scan` gelirse önceki iptal edilir; tek bir aktif oturum olur. İptal edilen taramanın ağacı bırakılır.
- **Sıralama**: `children()` bağlı liste yürüyüşüdür. Son `(düğüm, sıralama)` için sıralı `Vec<NodeId>` önbelleğe alınır, böylece aynı klasörün sonraki sayfaları O(limit) olur. 100k çocuklu bir klasör için test ve ölçüm eklenir.
- **Komut gövdeleri düz fonksiyonlardır** (`session.rs`). `#[tauri::command]` sarmalayıcıları ince kalır, böylece kayıt, sayfalama ve sıralama Tauri runtime'ı olmadan `cargo test` ile test edilir.
- **i18n**: bağımlılık eklenmez. Tipli bir TR/EN sözlük ve `t()` fonksiyonu kullanılır; anahtar eksikse `tsc` hata verir. İlk UI PR'ında kurulur ki sonradan string toplamak gerekmesin.
- **Güvenlik**: fs eklentisi yok, CSP olduğu gibi kalır, capability `core:default` olarak kalır. FDA ayar bölmesi Rust tarafında sabit bir `x-apple.systempreferences:` URL'i ile açılır; UI'dan URL alınmaz.

#### Tests
| Test | Neyi sabitler | PR |
|---|---|---|
| `node_id_round_trips_index` / `node_id_rejects_out_of_range` | core erişimcisi | 1 |
| `children_page_sorted_by_allocated_desc` | sıralama + DTO | 1 |
| `children_pagination_offset_limit` | sayfalama, son sayfa, aralık dışı offset | 1 |
| `stale_scan_id_is_rejected` | eski oturum kimliği | 1 |
| `unknown_node_is_rejected` | aralık dışı indeks | 1 |
| `cancel_drops_tree_and_reports_cancelled` | iptal | 1 |
| `new_start_cancels_running_scan` | tek oturum | 1 |
| `ancestors_are_root_first` | breadcrumb | 1 |
| `sort_100k_children_pages_fast` | önbellek (ölçüm/sınır) | 1 |
| `ipc_ts_mentions_every_serialized_key` | ipc.ts her serileşen anahtarı ve etiketi içerir (alt dize kontrolü; tip uyuşmazlığını yakalamaz) | 1 |
| `list_volumes_contains_root_mount` | birim listesi (Unix'te `/`, Windows'ta `C:\`) | 2 |
| UI: `pnpm typecheck` + `lint` + `build` | eksik çeviri anahtarı, tipler | 3–5 |
| Smoke: `pnpm tauri dev` ile gerçek pencerede birim → tara → iptal → tekrar tara → drill-down → breadcrumb | M2 kabul ölçütleri | 3–5 |

UI birim testi için test runner (vitest) **eklenmiyor**; ihtiyaç doğarsa ayrı onayla eklenir.

#### Kararlar (2026-11-14 onayı)
- Plan onaylandı. Core API eki `Tree::node_id` onaylandı.
- `tauri-specta` hâlâ RC (2.0.0-rc.29). Bu yüzden TS tipleri elle yazıldı (`ui/src/lib/ipc.ts`). Sözleşme kayması `ipc_ts_mentions_every_serialized_key` testiyle yakalanır; testin, yeniden adlandırılan bir anahtarı yakaladığı doğrulandı.
- CLI Ctrl-C tekrar Not Now'a alındı. shadcn M2'de eklenmiyor. UI PR'larından (3–5) önce `/design-brief` + `/direction` çalıştırılacak.
- PR 2 için `sysinfo` onaylandı (yalnızca `disk` özelliği). 0.39+ MSRV 1.95 istiyor, workspace 1.90'a sabit. Bu yüzden `0.38` (MSRV 1.88) kullanılıyor.
- macOS'ta `/System/Volumes/*` bağlama noktaları listede gizlenir (Finder da gizler). Kullanıcı yalnızca "Macintosh HD" (`/`) görür.
- Tasarım yönü A "Döküm fişi" onaylandı. PR 3 için `zustand` onaylandı. Erişilebilirlik hedefi WCAG 2.1 AA.

#### PR 3 planı (`feat/m2-ui-shell`)
- **Tokenlar** `ui/src/index.css` içinde CSS değişkeni olarak (DESIGN_MAP → Tokens). Tema `prefers-color-scheme` ile gelir; `<html data-theme>` ile zorlanabilir (koyu ekran görüntüsü için).
- **Durum** `ui/src/store.ts` (zustand): ekran ayrık bir birleşimdir: `volumes | scanning | done | cancelled | failed`. Eski taramanın olayları nesil sayacıyla yok sayılır.
- **i18n** `ui/src/lib/i18n.ts`: tipli TR/EN sözlük, `t()`; EN'de eksik anahtar `tsc` hatasıdır. Dil sistemden gelir, başlıktaki düğmeyle değişir.
- **Biçim** `ui/src/lib/format.ts`: boyutlar ondalık birimle (1 GB = 10⁹ bayt, Finder gibi), sayılar ve süre `Intl` ile yerele göre.
- **Ekranlar**: birim kartları (doluluk çubuğu; tarama bitince en büyük kalemlere bölünür), canlı sayaçlar + iptal (Esc), döküm (en büyük 5 kalem, diğerleri, toplam, sayılmayanlar dipnotu). Klasör seçici yok (dialog eklentisi gerekir); ağaç tablo PR 4.

#### Durum
- [x] PR 1 `feat(app): scan session IPC`: #9 merge edildi.
- [x] PR 2 `feat(platform): volume list`: #10 merge edildi.
- [x] Tasarım yönü: `/design-brief` + `/direction` tamam, yön A "Döküm fişi" onaylandı (ui/DESIGN_MAP.md).
- [x] PR 3 `feat(ui): shell, volumes, scan screen`: #12 merge edildi. İptal takılması #13 ile düzeldi.
- [x] PR 4 `feat(ui): tree table`: #15 içinde merge edildi (#14 kapatıldı, içeriği #15'te).
- [ ] PR 5 `feat(app): macOS Full Disk Access onboarding` (Notion #22): plan onaylandı ("Onaylıyorum, başla") ve uygulandı (`feat/m2-fda-onboarding`).
- [x] Tasarım geçişi: #15 merge edildi (2026-11-15, "A + B önerilerimle"). C fikirleri bu PR'a girmedi.
- [x] Canlı tarama sayaçları (leader satırları) paketli uygulamada görsel olarak doğrulandı (PR 5 smoke testi).

#### PR 5 planı (`feat/m2-fda-onboarding`, Notion #22)
**Kabul**: macOS'ta Tam Disk Erişimi (FDA) yoksa birimler ekranında kullanıcıyı engellemeyen bir panel nedenini anlatır. "Sistem Ayarları'nı aç" düğmesi doğru bölmeyi açar. Kullanıcı ayarlardan döndüğünde durum kendiliğinden yeniden kontrol edilir; erişim verildiyse panel kaybolur. Windows ve Linux'ta hiçbir şey görünmez.

- **Rust, `src-tauri/src/access.rs` (yeni)**:
  - `disk_access() -> DiskAccess` döner: `Granted | Denied | Unknown | NotApplicable`.
    - Uygulamada: alanı olmayan bir enum olduğu için `kind` etiketi yerine düz dize olarak serileşir (`"granted"`, `"notApplicable"` …). `AccessError` `kind` etiketlidir.
  - **Tespit sezgiseldir.** Yalnızca FDA ile okunabilen dosyalar `File::open` ile açılır; içerik okunmaz.
    - Önce `/Library/Application Support/com.apple.TCC/TCC.db`, sonra `~/Library/Safari`.
    - `Ok` → `Granted`. `PermissionDenied` → `Denied`. `NotFound` veya başka bir hata bir sonraki adaya geçer. Hiçbiri karar vermezse `Unknown`.
  - Hata türünden duruma çeviren saf bir `classify(io::ErrorKind)` fonksiyonu vardır. Gerçek TCC durumu testte kurulamayacağı için birim testleri bunun üzerinde yapılır.
  - `open_disk_access_settings() -> Result<(), AccessError>`: sabit `x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles` URL'iyle `/usr/bin/open` çalıştırır. UI'dan URL ya da argüman alınmaz. macOS dışında `Unsupported` döner.
- **`lib.rs`**: iki ince `#[tauri::command]` sarmalayıcısı eklenir. Capability (`core:default`) ve CSP değişmez. Yeni bağımlılık yok.
- **UI**:
  - `lib/ipc.ts`: `DiskAccess` tipi ve iki çağrı.
  - `store.ts`: `access` durumu. Açılışta ve pencere odağa döndüğünde (`focus` olayı, kullanıcı ayarlardan geri geldiğinde) yeniden kontrol edilir.
  - `screens/Volumes.tsx`: yalnızca `Denied` olduğunda birim listesinin üstünde bir panel gösterilir.
    - Tek cümleyle neden: korumalı klasörler (Mail, Safari, başka uygulamaların verileri) okunamaz, toplam eksik kalır.
    - Birincil düğme: "Sistem Ayarları'nı aç". Sessiz metin düğmesi: "Şimdilik geç".
    - Not: erişim verildikten sonra macOS uygulamayı yeniden açmayı isteyebilir.
    - Tarama engellenmez.
    - "Şimdilik geç" seçimi `localStorage`'da tutulur. `Unknown` durumunda panel gösterilmez; sezgisel tespit yanlış negatif verirse kullanıcı rahatsız edilmez.
  - `screens/Summary.tsx`: `Denied` iken "Sayılmayanlar" altındaki "okunamayan N öğe" satırına tek bir cümle eklenir: "Tam Disk Erişimi verilmedi" ve ayarları açan bir bağlantı. Panel kapatılmış olsa bile bu ipucu kalır.
  - `lib/i18n.ts`: TR/EN anahtarları.
- **Tasarım**: yön içinde kalır. Panel mevcut `rounded-panel border-hairline bg-card` kartıdır. Yeni token ya da ortak bileşen değişikliği yok; mevcut `Button` kullanılır. Uyarı rengi, ikon ya da kırmızı yok; bu bir hata değil, bir eksiklik notu.
- **Dokunulacak dosyalar**:
  - Rust: `src-tauri/src/access.rs` (yeni), `src-tauri/src/lib.rs`.
  - UI: `ui/src/lib/ipc.ts`, `ui/src/store.ts`, `ui/src/screens/Volumes.tsx`, `ui/src/screens/Summary.tsx`, `ui/src/lib/i18n.ts`.
- **Testler**:
  - `classify_maps_permission_denied_to_denied`, `classify_skips_not_found`.
  - `non_macos_is_not_applicable` (`cfg(not(target_os = "macos"))`).
  - `disk_access_serializes_kind_tag`.
  - `ipc_ts_mentions_every_serialized_key` testi `DiskAccess`'i de kapsayacak şekilde genişletilir.
- **Doğrulama**:
  - cargo zinciri ve `pnpm typecheck/lint/build`.
  - Paketli `Cleany.app` ile smoke test: FDA kapalıyken panel görünür ve düğme doğru Ayarlar bölmesini açar. FDA verilip uygulamaya dönülünce panel kaybolur. Bu iki adımı kullanıcının yapması gerekir, çünkü FDA'yı yalnızca kullanıcı açıp kapatabilir.
  - Koyu/açık ve TR/EN ekran görüntüleri.
- **Bilinen sınırlama**: `pnpm tauri dev` altında TCC izni uygulamaya değil Terminal'e aittir. Gerçek davranış yalnızca paketli `.app` ile doğrulanır.

Sorular:
1. Notion panosu geride kalmış: #5, #18, #19, #20, #21 ve #23 hâlâ Backlog, Devam ediyor ya da İncelemede görünüyor, ama işleri #9–#15 ile merge edildi. Bunlar "Bitti" yapılsın mı (önerim: evet)?
2. Dökümdeki FDA ipucu bu PR'a girsin mi (önerim: evet; okunamayan öğeleri en çok o anda fark ediyorsun)?

#### Tasarım geçişi: denetim bulguları ve düzeltme planı (2026-11-15)
**Kaynak**: paketli uygulamadan 11 ekran görüntüsü alındı (koyu/açık, TR/EN; birimler, tarama, döküm, açılmış döküm, ağaç tablo). Kit'in `/slop-audit`, `/interface-check` ve `/stress-ui` denetimleri ile Fable'ın bağımsız tasarım eleştirisi bu görüntüler ve kod üzerinden yapıldı. Kontrastlar WCAG formülüyle **ölçüldü**: ink-3 yalnızca raised zeminde kalıyor (4,17–4,27), geri kalan her şey AA'yı geçiyor.

**Genel teşhis**: slop yok denecek kadar az (gradyan, glow, emoji, hype yok). Sorun silik uygulama. Yön doğru tanımlanmış ama çekingen uygulanmış:
- Dökümde gözün ineceği bir nokta yok. Cevap rakam (toplam), ama en altta 15px kalıyor.
- Toplam üç kez yazılıyor.
- Döküm ekranında birincil eylem yok.
- Leader çizgisi küçük ölçekte noktalı değil düz bir çizgi gibi okunuyor, altındaki ShareBar ile çift alt çizgi oluşturuyor.
- 1200×800 pencerenin yarısı boş.
- Tarama ve tablo ekranları herhangi bir rakibin ekranı olabilir.

**Görünen ad**: `tauri.conf.json` içinde `mainBinaryName: "Cleany"` ayarlandı (pencere, menü, .app ve binary "Cleany"; Cargo paketi içeride `cleany-app` kalır). Commit edilmedi, bu PR'a girecek.

##### A. Yön içinde (ekran düzeyi, onay sonrası doğrudan uygulanır)
1. **Döküm: cevap rakam olsun.** Toplam yalnızca bir kez, alt toplam çizgisinin altında yazılır (Moments), ama `text-figure` (28/500, mono) olur. Lede'den "404 MB" çıkar, yalnızca "Birimde 389 MB boş." kalır.
2. **Döküm: tek birincil eylem.** "Tümüne göz at" birincil (accent) olur ve ilk sıraya geçer. "Birimlere dön" kaldırılır (başlıktaki geri bağlantısı aynı işi yapıyor). "Yeniden tara" sessiz metin düğmesi olarak sağa alınır.
3. **Döküm: satır düzeni.** ShareBar adın altından kendi sütununa taşınır (ad ··· boyut | çubuk 120px | pay). Çift alt çizgi kalkar. %0,5'in altındaki kalemlerde çubuk gösterilmez, böylece 1px "toz" çizgiler kaybolur.
4. **Döküm: iki eylem, tek işaret.** Şu an chevron yerinde açıyor, ad ise başka ekrana götürüyor ve bunu gösteren hiçbir şey yok. Satırın tamamı yerinde açar. Hover ve odakta görünen "Aç →" bağlantısı tabloya götürür.
5. **Döküm: dipnot `*` işaretleri** hiçbir şeye bağlanmıyor. "Sayılmayanlar" başlığı altında düz liste olur.
6. **Tarama: ilk ilerleme gelmeden** "— bulundu, yaklaşık 450 MB içinden" kırık gibi okunuyor. İlerleme gelene kadar yalnızca "Tarama başlıyor…" gösterilir. Sonra rakam 28px, yanında "yaklaşık 450 MB içinden". Canlı sayaçlar fişe satır satır yazılır gibi leader satırlarına dönüşür: Dosya ···· 100.092 / Klasör ···· 12 / Süre ···· 7 sn. (DESIGN_MAP leader'ı döküm, sepet ve breadcrumb için sayıyor; tarama ekranına genişletmek küçük bir onay ister, aşağıda B4.)
7. **Teknik yol önde değil.** Tarama başlığının altındaki mono `/Volumes/CleanyTree` kaldırılır. Birim kartında yalnızca aynı adlı iki birim varsa gösterilir, diğer durumlarda `title` içinde kalır (Audience: teknik ayrıntı arka planda).
8. **Tablo: genel araç görünümü.**
   - Ad sütunu sınırlanır, böylece ad ile boyut arasında 300px boşluk kalmaz.
   - Kart, içeriği kadar uzar.
   - Genişlik `max-w-240` olur.
   - Kökte boş breadcrumb `nav`'ı render edilmez.
   - Sıralama veya klasör değişince eski satırlar korunur ("İçerik okunuyor…" yanıp sönmez).
9. **Gezinme tek yerde.** Tabloda "< Birimler" ve "< Döküm" alt alta duruyor. Başlık çubuğu tek breadcrumb olur: `Birimler / CleanyTree dökümü / Projeler`, her parça bağlantı. Sayfa içi geri düğmesi kalkar. Aynı başlık döküm ve tarama ekranında da kullanılır.
10. **Pencere kullanımı.** Döküm dar kalır (fiş dardır). ≥1100px'te iki sütun olur:
    - solda fiş (~560px);
    - sağda (~280px) sayılmayanlar dipnotu, tarama istatistiği ve eylemler.
    Birimler ve tarama ekranlarında üst boşluk `py-16` olur.
11. **Birim kartı (Moment 1).** Bölünmüş doluluk çubuğu 6px'ten 12px'e çıkar, bölümler arası 2px boşluk olur. Altındaki "Çubuk … bölündü" açıklaması yerine en büyük 3 kalem `text-meta` leader satırı olarak yazılır. "Son tarama: 2 dk önce" eklenir. Birincil eylem, çiftin solunda.
12. **Tek fiil, tek yazı yüzü.**
    - Fiiller: "Yeniden tara" her yerde (şu an "Tekrar tara" da var). Tabloya giden eylem TR'de "Tümüne göz at", EN'de "Browse all" olarak tutarlı.
    - Yazı yüzü: birim kartındaki 28px rakam ve "927 GB / 995 GB" satırı da mono olur, uygulamadaki her bayt değeri tek yüzde.
13. **Erişilebilirlik ve dayanıklılık** (interface-check, stress-ui):
    - Breadcrumb, sıralama başlıkları ve `<summary>` için tıklama hedefi ≥24px (`min-h-6`).
    - Birim düğmelerinin aria-label'ı görünen metinle başlar ("Dökümü aç: CleanyTree").
    - `aria-live` tüm listeyi değil, yalnızca küçük bir `role="status"` öğesini kapsar.
    - Tarama ilerlemesi ekran okuyucuya seyrek bir `role="status"` satırıyla duyurulur.
    - Uzun adlar ortadan kısaltılır (sonu kaybolmasın); tabloda seçili satırın tam adı alt satırda görünür.
    - Sayfa başlığında yalnızca ad kısaltılır, fiil kalır ("…dökümü").
    - Adlar `<bdi>` içine alınır.
    - Dar pencerede/%200 yakınlaştırmada tablo Pay/Dosya/Değiştirilme sütunlarını gizler.
    - Hata ekranında ham hata metninin üstüne düz bir cümle eklenir.

##### B. Onay gerekenler (token, ortak bileşen veya DESIGN_MAP)
1. **Leader'ın okunurluğu**: token `1px dotted`. Önerim 2px noktalar, 6px aralık (`radial-gradient` ile, renk ink-3). Ekran görüntüsünde fişin imzası şu an görünmüyor. *Önerim: evet.*
2. **Ara aralık adımları**: kodda 2px/6px kullanımları var (çubuk yüksekliği, etiket dolgusu). Ya ölçeğe yuvarlanır ya da 2 ve 6 haritaya eklenir. *Önerim: 2'yi ekle (ince çubuk/boşluk), 6'yı yuvarla.*
3. **Ortak bileşenler** (`ui.tsx`): ShareBar (eşik + yeni yer), LeaderRow, UsageBar değişir. A3, A11 ve B1 bunlara bağlı.
4. **Leader tarama ekranında**: A6 için Signature satırına "canlı sayaçlar" eklenir. *Önerim: evet, fiş satır satır yazılıyor fikri yönü güçlendiriyor.*
5. **Birim çubuğunda en büyük bölüme accent**: şu an accent yalnızca dökümdeki en büyük çubukta. *Önerim: hayır, birim kartı sessiz kalsın.*
6. **%1 altı kalemleri "Diğer"e katlamak** (`.fseventsd`, `.DS_Store` gibi): onaylı "en büyük 12" kuralını değiştirir. *Önerim: hayır. 12 kalsın, A3'teki çubuk eşiği gürültüyü zaten azaltıyor.*

##### C. Cesur fikirler (isteğe bağlı, Fable; bu PR'a girmez)
- **Döküm fiziksel bir fiş olsun**: ~480px dar kart, üstte ve altta tırtıklı kenar (CSS mask, gradyan yok), başlıkta mono "CleanyTree · 15 Kas 2026 · 7,1 sn". World "paneller nötr kalır" dediği için yön değişikliği sayılır.
- **Döküm ile tablo tek defter olsun**: klasöre inmek o klasörün fişini yeniden keser, yoğun tablo "N öğenin tümü" modu olur. Signature tabloda leader'ı yasaklıyor, yani yön değişikliği.

##### Uygulama
- **Dal**: `feat/m2-design-pass`, tek PR.
- **Dokunulacak dosyalar**: `screens/{Summary,Scanning,Volumes,Browse,Ended}.tsx`, `components/ui.tsx`, `index.css` (yalnızca B onaylanırsa), `App.tsx` (başlık breadcrumb), `lib/i18n.ts`, `store.ts` (döküm açılım durumu korunur), `DESIGN_MAP.md` (B onaylarını kaydetmek için), `src-tauri/tauri.conf.json`.
- **Rust**: değişiklik yok.
- **Doğrulama**: typecheck, lint, build, cargo zinciri. Paketli uygulamada koyu+açık, TR+EN ekran görüntüleri ve önce/sonra karşılaştırması. Klavyeyle tam tur. Uzun ad ve 100k klasörüyle stres. Ardından `/slop-audit` tekrar.

#### PR 4 planı (`feat/m2-tree-table`, Notion #21)
- **Giriş noktası**: dökümde bir kalemin adına tıklanınca ağaç tablo o klasörde açılır. "Tümüne göz at" bağlantısı tabloyu kökte açar. Döküm Moment 2 olarak yerinde kalır, tablo dökümün yerine geçmez. Yeni bir ekran türü eklenir: `{ kind: 'browse'; done: DoneScreen; node; sort; order }`. Geri dönüşte döküm açılır.
- **Breadcrumb**: `getAncestors` ile kökten seçili klasöre kadar parçalar, en sonda toplam (DESIGN_MAP: leader çizgisi breadcrumb toplamında kullanılır). Her parça tıklanabilir.
- **Tablo**: sütunlar Ad · Boyut (allocated) · Pay (üst klasöre göre) · Dosya · Değiştirilme. Satır yüksekliği 32px. Rakamlar sağa hizalı, `tabular-nums` ve mono. Leader yok. Başlığa tıklanınca o sütuna göre sıralanır (ikinci tıklama yönü çevirir). Sıralama `aria-sort` ile duyurulur.
- **Sanal liste**: satırlar 32px sabit olduğu için pencereleme elle yazılır: `scrollTop / 32` aralığı ve ±10 satır tampon. **Bağımlılık eklenmez.** Satırlar 200'lük sayfalar halinde, görünür aralık geldikçe `getChildren` ile alınır. Önbellek klasör + sıralama ile anahtarlanır, sıralama değişince sıfırlanır.
- **Gezinme**: tek tık ya da Enter klasörün içine girer (ADR-0004). Dosyalar seçilir ama içine girilmez. Klavye: ↑/↓ satır, Home/End, PageUp/PageDown, Enter/→ içine gir, Backspace/← üst klasör, Esc dökümüne dön. Tablo ARIA `grid` değil, `aria-activedescendant` kullanan bir `listbox`/`table` olur. Odak halkası görünür.
- **Durumlar**: yükleniyor, hata, boş klasör ve `StaleScan` (yeni tarama başladıysa birimlere döner). 100k çocuklu bir klasörde kaydırmanın akıcı olduğu gerçek uygulamada kontrol edilir.
- **Dokunulacak dosyalar**: `store.ts` (browse ekranı), `screens/Browse.tsx` (yeni), `screens/Summary.tsx` (giriş bağlantıları), `App.tsx` (yönlendirme), `lib/i18n.ts` (TR/EN anahtarları), `lib/format.ts` (tarih biçimi). Rust tarafında değişiklik yok.
- **Doğrulama**: typecheck/lint/build, cargo zinciri, paketli uygulamada açık+koyu ekran görüntüsü, klavyeyle gezinme ve 100k çocuklu bir test klasörü.

Sorular:
1. `@tanstack/react-virtual` yerine elle pencereleme (önerim: satırlar sabit 32px olduğu için ~40 satır kod yeter, bağımlılık gerekmez)?
2. Giriş noktası olarak dökümden tıklama ve "Tümüne göz at" uygun mu? Yoksa tablo, tarama bitince doğrudan dökümün altında mı gösterilsin?

#### Open Questions
_2026-11-14: 1 → yön A; 2 → `sysinfo`, `zustand` onaylandı, `tauri-specta` alınmadı; 3, 4 cevaplandı (bkz. Kararlar)._
1. **Tasarım yönü (bloklayıcı)**: UI PR'larından (3–5) önce `ui/` içinde `/design-brief` + `/direction` çalıştırılıp yön onaylansın mı (önerim)? Alternatif: UI'yi nötr, yer tutucu stille kurup yönü sonra uygulamak. Bu durumda `ui/CLAUDE.md` kuralı bilerek esnetilmiş olur. PR 1 ve 2 bu cevabı beklemeden başlayabilir.
2. **Bağımlılıklar** (tümü korumalı değişiklik):
   | Bağımlılık | Neden | Alternatif |
   |---|---|---|
   | `sysinfo` (`default-features = false`, `disk`) | Birim listesi + toplam/boş alan, `unsafe` olmadan üç platformda | `statvfs`/`GetDiskFreeSpaceEx` için `libc`/`windows-sys` + `unsafe` modülü |
   | `specta` + `tauri-specta` | TS tiplerini Rust'tan üretir, sözleşme kaymasını önler | Elle yazılmış `ipc.ts` + serde JSON snapshot testi (bağımlılık yok). tauri-specta'nın güncel kararlı sürüm durumu henüz doğrulanmadı, onaydan önce kontrol edilecek. |
   | `zustand` | Oturum/gezinme durumu, küçük | React context + `useReducer` (bağımlılık yok) |
   | `@tanstack/react-virtual` | 100k satırlık klasörde sanal liste | Elle sabit satır yüksekliğiyle pencereleme |
   | shadcn/ui | PLAN §3.5'te aday | Tasarım yönü belli olunca karar verilsin; M2'de eklenmesin (önerim) |
3. **Core public API ekleme**: `Tree::node_id(u32) -> Option<NodeId>` onaylanıyor mu?
4. **CLI Ctrl-C** (M1'den M2'ye ertelenmişti, Notion kartı yok): `ctrlc` bağımlılığıyla küçük bir PR mı yapılsın, yoksa tekrar Not Now'a mı alınsın (önerim)?

#### Risks
- DESIGN_MAP onayı gecikirse UI PR'ları bekler; backend iki PR'ı bu sürede ilerler.
- `sysinfo` ağır bir crate; yalnızca `disk` özelliğiyle derleme süresi ve binary boyutu ölçülecek.
- FDA tespiti sezgiseldir (korumalı bir dizini okumayı denemek). macOS sürümüne göre değişebilir; yanlış negatifte kullanıcıyı bloklamayan bir uyarı gösterilir.
- Kök tarama (`/`) 1M+ düğüm üretir. Bellek M1'de ölçüldü, ancak oturumda tek ağaç tutulduğundan ikinci tarama birincisini bırakmalı (test: `new_start_cancels_running_scan`).

#### Not Now

- 375 px genişlikte başlıktaki "Ayarlar" metni konum satırını çok kısaltıyor. Masaüstü penceresinin en küçük genişliği 900 px olduğu için gerçek uygulamada görülmüyor. Gerekirse dar ekranda simge kullanılabilir (paylaşılan simge seti onay ister).
- Treemap, breakdown, top files, arama, reveal/open/trash/delete (M3+, PLAN §3.4).
- Birden fazla eşzamanlı tarama/sekme.
- **Tarama sonucu kalıcı değil**: her tarama sıfırdan yürür; sonuç yalnızca oturum belleğinde (tek ağaç). Son taramayı diske yazıp açılışta göstermek ve FSEvents (macOS) / USN günlüğü (Windows) ile artımlı güncelleme yapmak ayrı bir iş (M7 hızlı yollarla birlikte ya da sonrası).
- **macOS'ta `/` taraması Data birimini iki kez sayabilir**: `/`, `/Users` ve `/System/Volumes/Data` aynı `dev` değerini döndürüyor (APFS volume group). `cross_mounts=false` sınırı bu yüzden durmuyor. `/Users` firmlink üzerinden bir kez, `/System/Volumes/Data/Users` üzerinden bir kez daha yürünür. Çözüm core tarayıcıda olmalı (kök `/` iken `/System/Volumes/Data` atlanmalı). **Ölçüldü (2026-11-15, PR 5 smoke testi)**: 928 GB dolu Macintosh HD taraması toplam 1,5 TB verdi; `System` 804 GB, `Users` 599 GB. Çift sayım doğrulandı.

---

### M1 — Çekirdek tarayıcı + CLI (Notion #8–#17) ✅ 2026-11-14 (#4–#8 merged)

**Hedef**: `cleany scan <yol>` macOS/Windows/Linux'ta doğru toplam verir (Unix'te `du` ile ±%1, allocated). Hardlink, symlink ve sparse testleri geçer. 1M dosya < 150 MB RAM.

**Bağlam**: docs/PLAN.md §3.1–3.3, §9. M1'de yalnızca `std::fs` kullanılır. getattrlistbulk, statx ve MFT hızlı yolları M7'ye kalır.

#### PR bölümü
| PR | Kartlar | İçerik | Yeni bağımlılık |
|---|---|---|---|
| 1 `feat(core): arena tree` ✅ #4 | #8, #10 | `Tree`/`Node`/`NodeId`, ad havuzu, alt→üst toplama, `Size` (logical/allocated), ADR-0005 (crate yönü) | core: `thiserror` (workspace'te zaten var) |
| 2 `feat(core): parallel scanner` ✅ #5 | #9, #12, #14 | rayon havuzu, iptal, ≤10 Hz ilerleme, symlink/junction takip yok, mount sınırı, izin reddi listesi, OsString; seri referans yürüyücü + fikstürler; platform: std'den `EntryMeta` | core: `rayon`; dev: `tempfile` |
| 3 `feat(cli): cleany scan` ✅ #6 | #15, #16 | `cleany scan <yol> [--json] [--cross-mounts]` (argümanlar elle parse edilir), `du` karşılaştırma testi | cli: `serde_json` (workspace'te var) |
| 4 `feat(platform): hardlinks + cloud` ✅ #7 | #11, #13 | (dev,inode)/FileId tekilleştirme, iCloud/OneDrive yer tutucu algılama, Windows allocated boyut | Windows'a özel: aşağıdaki A/B seçeneğine göre |
| 5 `perf: bench` ✅ #8 | #17 | criterion, sentetik 1M ağaç, tepe RSS ölçümü | dev: `criterion` |

#### Tasarım kararları
- **Crate yönü**: `cleany-core → cleany-platform`. Platform, ham `u64`'lerle `EntryMeta { kind, logical, allocated, dev, file_id, nlink, mtime, flags }` döndürür. `Size`, ağaç ve tarayıcı core'da kalır. ADR-0005 olarak yazılır.
- **Node**: `parent`, `first_child`, `next_sibling`, `name_off` (u32), `name_len` (u16), `kind/flags` (u16), `logical`, `allocated` (u64), `file_count` (u32), `mtime` (i64) → hedef ≤48 bayt (test bunu doğrular).
- **Ad havuzu**: Unix'te bayt havuzu (`OsStrExt::from_bytes`), Windows'ta u16 havuzu (`encode_wide`/`from_wide`). Böylece `unsafe` gerekmez ve dönüşüm kayıpsızdır.
- **Paralellik**: rayon işçileri dizinleri okuyup `DirListing`'i kanala gönderir. Arenaya yalnızca tek bir builder thread yazar, ID'leri o atar ve alt dizinleri yeniden kuyruğa koyar. İptal `AtomicBool` ile yapılır, ilerleme builder'da 100 ms throttle'lanır.
- **Klasör boyutu**: dizinin kendi blokları da toplama dahil edilir (`du` ile tutması için).
- **Hardlink**: yalnızca `nlink > 1` olan girdiler izlenir. Bir grubun baytlarının sahibi deterministiktir (yol sırasına göre en küçük olan); diğer kopyalar toplama 0 katar ve `HARDLINK` bayrağı taşır.
- **Kök symlink ise** (macOS `/tmp` gibi): bir kez çözülür, altındaki hiçbir link takip edilmez.
- **Bulut yer tutucu**: macOS'ta `st_flags & SF_DATALESS` olan dizine **inilmez**, çünkü listelemek indirmeyi tetikleyebilir. Dosyanın içeriği hiç okunmaz. Windows'ta `RECALL_ON_DATA_ACCESS/RECALL_ON_OPEN/OFFLINE` öznitelikleri kontrol edilir. Gerçek yer tutucu fikstürlenemediği için sınıflandırma fonksiyonu ham değerlerle test edilir.

#### Tests
| Test | Neyi sabitler | Gating |
|---|---|---|
| `node_is_at_most_48_bytes` | bellek hedefi | hepsi |
| `aggregate_sums_children_bottom_up` | toplama | hepsi |
| `name_pool_roundtrips_non_utf8` | OsString kayıpsız | unix (geçersiz UTF-8) + windows (eşleşmemiş surrogate) |
| `parallel_matches_serial_reference` | paralel = seri | hepsi |
| `cancel_stops_scan` | iptal | hepsi |
| `progress_is_throttled` | ≤10 Hz | hepsi |
| `symlink_loop_is_not_followed` | sonsuz döngü yok | unix |
| `junction_is_not_followed` | reparse | windows (`mklink /J`) |
| `permission_denied_is_reported_and_scan_continues` | izin raporu | unix; root olarak `read_dir` başarılı olursa kendini atlar |
| `sparse_file_allocated_less_than_logical` | sparse | FS destekliyorsa (kendi tespit eder) |
| `hardlinks_counted_once_deterministically` | tekilleştirme | hepsi (Windows `fs::hard_link`) |
| `placeholder_flags_classified` | bulut | macos + windows (ham değerler) |
| `du_total_matches_within_1_percent` | referans | unix, integration (`cargo test` içinde, CI dosyası değişmez) |
| RSS kontrolü (1M dosya) | <150 MB → ölçülen 58–64 MiB (docs/PERF.md) | manuel, release CLI + `/usr/bin/time` |

#### Open Questions
_2026-11-14: Plan, bağımlılıklar ve Windows **A seçeneği** onaylandı._
1. **Windows `unsafe` kararı (PR 4)** → **A seçildi**:
   - **A (önerim)**: güvenli wrapper crate'ler: `winapi-util` (FileId, volume serial, nlink) + `filesize` (GetCompressedFileSizeW). M1'de `unsafe` yok.
   - **B**: `windows-sys` + `cleany-platform` içinde ADR'li küçük bir `unsafe` modülü. M7'deki MFT yolu için zaten gerekecek.
2. Yeni public API (core): `scan(root, &ScanOptions, &CancelToken, on_progress) -> Result<ScanResult, ScanError>`, `Tree::{root, node, children, name, path}`, `ScanResult { tree, issues, stats }`.

#### Risks
- Windows'ta FileId/nlink için her dosyada handle açmak yavaştır. Kabul edilebilir, çünkü M7'de MFT yolu bunun yerini alır.
- APFS klonları `du` ile de sapabilir. Fikstür klon içermez.
- macOS CI runner'ında Full Disk Access yok. Testler yalnızca tempdir'de çalışır.

#### Not Now
- getattrlistbulk / statx / MFT hızlı yolları (M7), Tauri komutları (M2).
- `cleany scan` Ctrl-C ile iptal: sinyal yakalamak için yeni bağımlılık (`ctrlc`) ya da `unsafe` gerekir; M2'de UI iptali ile birlikte ele alınacak.
- `~/Library` taraması 71 sn sürdü; PR 4'teki yer tutucu algılamasından sonra 26 sn (hâlâ ağırlıklı sistem zamanı, %282 CPU). 1M dosyalık ölçümde de süre çekirdekte (`readdir`/`lstat`) geçiyor (docs/PERF.md); M7 hızlı yolları ele alacak.

---

## Not Now

Parked scope — deferred work, revisit when prioritized. (CLAUDE.md → Scope Discipline routes out-of-scope items here.)

- **Macintosh HD toplamı diski aşıyor** (2026-11-16, #30 denemesi sırasında): tarama 1,6 TB gösterdi (System 833 GB, Users 626 GB), disk ise 953 GB dolu; ilerleme çubuğu da "yaklaşık 953 GB" tahminini geçti. `cross_mounts: false` ve "20 diğer birim dışarıda" olmasına rağmen Data birimi `/System/Volumes/Data` ve firmlink'ler üzerinden iki kez sayılıyor olabilir. Ayrı hata; #30 kapsamı dışında.
- **Taramanın disk önbelleği** (2026-11-16, canlı tarama planından ertelendi): yalnız süreç yeniden başlamasını çözer; doğru olması için FSEvents `sinceWhen` geriye oynatması gerekir. v1.x "Anlık görüntü ve fark" ile birlikte ele alınmalı. Linux (`inotify` izleme sınırı) ve Windows canlı izleme de ayrı iş.
- **Silme logunu uygulamadan göster** (#32'den ertelendi): "Logu göster" düğmesi `deletions.jsonl` dosyasını dosya yöneticisinde açabilir; yeni IPC komutu gerektirdiği için #32 kapsamı dışında bırakıldı.
- **Döküm "Aç →" dokunmatikte görünmüyor** (#28 interface-check, Minor): `ui/src/screens/Summary.tsx:309` yalnızca hover/odakta görünür; Tailwind v4 `group-hover` `(hover: hover)` ile kapılı olduğundan dokunmatik ekranda tıklanabilir ama görünmez. Masaüstü uygulamasında pratik etkisi yok; dokunmatik/Linux tablet desteği gündeme gelirse `@media (hover: none)` altında sürekli göster.
- **Canlı güncelleme firmlink kopyasına ulaşmıyor** (2026-11-17, PR 4 smoke testi; bilinen sınırlama): FSEvents yolları `/Users/...` olarak bildiriyor. `/` taramasında değişiklik Tümü › Users düğümüne işleniyor; aynı verinin System › Volumes › Data › Users kopyası eski kalıyor. Kök neden yukarıdaki çift sayım hatasıyla aynı (Data birimi iki yoldan yürünüyor); o düzeltilince (kök `/` iken `/System/Volumes/Data` atlanınca) bu da kapanır. Ayrı iş.
- **"Güncel değil" çubuğu tek metin kullanıyor** (2026-11-17, PR 4 smoke testi, Minor): taranan birim çıkarıldığında da "Tarama sonrasında diskte çok sayıda değişiklik yapıldı" yazıyor. "Yeniden tara" var olan hata ekranına düşüyor ("cannot read scan root"), işlev doğru. Nedene göre metin (kök kayboldu / çok değişiklik) için `outdated` olayına neden alanı eklenmeli; IPC değişikliği olduğundan ayrı iş.
- **Görünüm seçici vurgusu arka plan penceresinde güncellenmedi** (2026-11-17, #34 smoke testi, doğrulanmadı): release uygulamasında Harita ve Halka açıldığı halde "Tablo" düğmesi seçili renkte kaldı. Kod doğru (`ui/src/screens/Browse.tsx:361`, `view === v`). Pencere arka plandayken WebKit'in CSS geçişlerini (`transition-[background-color,color]`) duraklatması olası neden; CSP ile ilgisi yok. Öndeki pencerede elle kontrol edilmeli.
