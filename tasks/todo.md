# Task Board

Track current and upcoming tasks here. The agent updates this file as work progresses.

Give the task you are actively working on an `h3` heading under **In Progress** —
`session-start.sh` reads the first one and injects it as the session's active task.

---

## In Progress

### M1 — Çekirdek tarayıcı + CLI (Notion #8–#17)

**Hedef**: `cleany scan <yol>` macOS/Windows/Linux'ta doğru toplam verir (Unix'te `du` ile ±%1, allocated). Hardlink, symlink ve sparse testleri geçer. 1M dosya < 150 MB RAM.

**Bağlam**: docs/PLAN.md §3.1–3.3, §9. M1'de yalnızca `std::fs` kullanılır. getattrlistbulk, statx ve MFT hızlı yolları M7'ye kalır.

#### PR bölümü
| PR | Kartlar | İçerik | Yeni bağımlılık |
|---|---|---|---|
| 1 `feat(core): arena tree` | #8, #10 | `Tree`/`Node`/`NodeId`, ad havuzu, alt→üst toplama, `Size` (logical/allocated), ADR-0005 (crate yönü) | core: `thiserror` (workspace'te zaten var) |
| 2 `feat(core): parallel scanner` | #9, #12, #14 | rayon havuzu, iptal, ≤10 Hz ilerleme, symlink/junction takip yok, mount sınırı, izin reddi listesi, OsString; seri referans yürüyücü + fikstürler; platform: std'den `EntryMeta` | core: `rayon`; dev: `tempfile` |
| 3 `feat(cli): cleany scan` | #15, #16 | `cleany scan <yol> [--json] [--cross-mounts]` (argümanlar elle parse edilir), `du` karşılaştırma testi | cli: `serde_json` (workspace'te var) |
| 4 `feat(platform): hardlinks + cloud` ✅ PR açıldı | #11, #13 | (dev,inode)/FileId tekilleştirme, iCloud/OneDrive yer tutucu algılama, Windows allocated boyut | Windows'a özel: aşağıdaki A/B seçeneğine göre |
| 5 `perf: bench` | #17 | criterion, sentetik 1M ağaç, tepe RSS ölçümü | dev: `criterion` |

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
| RSS kontrolü (1M ağaç) | <150 MB | PR 5'te bench ile manuel/ölçüm notu |

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
- `~/Library` taraması 71 sn sürdü; PR 4'teki yer tutucu algılamasından sonra 26 sn (hâlâ ağırlıklı sistem zamanı, %282 CPU). Kalan maliyet PR 5'teki bench ile profillenecek.

---

## Up Next

---

## Done

---

## Not Now

Parked scope — deferred work, revisit when prioritized. (CLAUDE.md → Scope Discipline routes out-of-scope items here.)
