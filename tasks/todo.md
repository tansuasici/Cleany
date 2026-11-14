# Task Board

Track current and upcoming tasks here. The agent updates this file as work progresses.

Give the task you are actively working on an `h3` heading under **In Progress** —
`session-start.sh` reads the first one and injects it as the session's active task.

---

## In Progress

### M2 — Uygulama kabuğu (Notion #18–#23)

**Hedef**: Uygulama açılınca birimler (volume) listelenir. Kullanıcı bir birimi ya da klasörü tarar, canlı ilerlemeyi görür ve taramayı iptal edebilir. Sonuç sanal bir ağaç tabloda boyuta göre sıralı gelir; tek tıkla klasörün içine girilir (ADR-0004), breadcrumb ile geri çıkılır. UI hiçbir zaman milyonlarca düğüm almaz, yalnızca sayfa sayfa çocuk listesi alır.

**Bağlam**: docs/PLAN.md §3.4 (IPC), §3.5 (frontend), §9 (M2 kabul ölçütleri). Notion #5 (`/design-brief` + `/direction`) hâlâ "Devam ediyor" ve `ui/DESIGN_MAP.md` boş şablon. `ui/CLAUDE.md`, DESIGN_MAP boşken UI yazılmamasını söylüyor. Bu yüzden backend PR'ları önce gelir, UI PR'ları yön onayına bağlıdır.

#### PR bölümü
| PR | Kartlar | İçerik | Yeni bağımlılık (onay gerekir) | Tasarıma bağlı mı |
|---|---|---|---|---|
| 1 `feat(app): scan session IPC` | #18, #20 (backend) | Tek oturumluk tarama kaydı, `start_scan` / `cancel_scan` / `get_children` / `get_ancestors`, ilerleme `tauri::ipc::Channel` üzerinden (core zaten ≤10 Hz), DTO'lar, TS tipleri | `specta` + `tauri-specta` **veya** elle TS tipleri (Soru 3) | Hayır |
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
| `ts_bindings_are_up_to_date` | TS tipleri Rust ile senkron (specta ya da serde snapshot) | 1 |
| `list_volumes_contains_root_mount` | birim listesi (Unix'te `/`, Windows'ta `C:\`) | 2 |
| UI: `pnpm typecheck` + `lint` + `build` | eksik çeviri anahtarı, tipler | 3–5 |
| Smoke: `pnpm tauri dev` ile gerçek pencerede birim → tara → iptal → tekrar tara → drill-down → breadcrumb | M2 kabul ölçütleri | 3–5 |

UI birim testi için test runner (vitest) **eklenmiyor**; ihtiyaç doğarsa ayrı onayla eklenir.

#### Kararlar (2026-11-14 onayı)
- Plan onaylandı. Core API eki `Tree::node_id` onaylandı.
- `tauri-specta` hâlâ RC (2.0.0-rc.29). Bu yüzden TS tipleri elle yazıldı (`ui/src/lib/ipc.ts`). Sözleşme kayması `ipc_ts_mentions_every_serialized_key` testiyle yakalanır; testin, yeniden adlandırılan bir anahtarı yakaladığı doğrulandı.
- CLI Ctrl-C tekrar Not Now'a alındı. shadcn M2'de eklenmiyor. UI PR'larından (3–5) önce `/design-brief` + `/direction` çalıştırılacak.

#### Durum
- [x] PR 1 `feat(app): scan session IPC`: branch `feat/m2-scan-ipc`, PR açıldı, incelemede.
- [ ] PR 2–5

#### Open Questions
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

---

## Up Next

---

## Done

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
