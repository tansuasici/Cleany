# Task Board

Track current and upcoming tasks here. The agent updates this file as work progresses.

Give the task you are actively working on an `h3` heading under **In Progress** —
`session-start.sh` reads the first one and injects it as the session's active task.

---

## In Progress

### M4 — Göster / aç / yolu kopyala (Notion #29)

**Hedef**: Browse'un üç görünümünde (Tablo, Harita, Halka) seçili öğe için üç eylem: dosya yöneticisinde göster, varsayılan uygulamayla aç, tam yolu panoya kopyala. Bu eylemler düğmelerle ve klavyeyle yapılabilmeli. macOS, Windows ve Linux'ta çalışmalı. Notion kartı boş; kabul ölçütü bu satır.

**Bağlam**: docs/PLAN.md:44 (Aksiyonlar), :110 (`reveal(node)`, `open(node)`), :72 (`cleany-platform` = "çöp/reveal"). M4'ün ilk kartı; #33 (sepet) ve #30 (çöp) aynı "seçili öğe" bağlantısını kullanacak. Tek PR, dal `feat/m4-reveal-open-copy`.

#### Durum (2026-11-15)
Plan onaylandı; açık sorular: 1 `R`/`O`/`⌘C`, 2 evet, 3 `std::process::Command`, 4 bağlam menüsü yok. Kod tamam, commit yok.
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

## Up Next

---

## Done

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

- **Döküm "Aç →" dokunmatikte görünmüyor** (#28 interface-check, Minor): `ui/src/screens/Summary.tsx:309` yalnızca hover/odakta görünür; Tailwind v4 `group-hover` `(hover: hover)` ile kapılı olduğundan dokunmatik ekranda tıklanabilir ama görünmez. Masaüstü uygulamasında pratik etkisi yok; dokunmatik/Linux tablet desteği gündeme gelirse `@media (hover: none)` altında sürekli göster.
