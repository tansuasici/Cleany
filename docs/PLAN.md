# Cleany — Ürün ve Teknik Plan

> Durum: **Taslak / onay bekliyor** · Tarih: 2026-11-13
> Repo: https://github.com/tansuasici/Cleany · Lisans: açık kaynak (bkz. §11)

Cleany, macOS, Windows ve Linux'ta çalışan, diskteki alanın **nereye gittiğini detaylıca gösteren** ve gereksiz dosyaları **güvenle temizlemeyi** sağlayan, Tauri tabanlı açık kaynak bir masaüstü uygulamasıdır. Hedef: WizTree hızına, DaisyDisk görselliğine, GrandPerspective/ncdu doğruluğuna yaklaşan tek bir cross‑platform araç.

---

## 1. Dil kararı: Rust mı, Zig mi, C mi?

Önce gerçek soruyu doğru koyalım: **Tauri'nin çekirdeği zaten Rust.** Uygulama kabuğu, IPC, pencere yönetimi, updater, plugin'ler hep Rust. Dolayısıyla seçenekler aslında şunlar:

| | A) Sadece Rust | B) Rust + Zig tarayıcı (FFI) | C) Rust + C tarayıcı (FFI) |
|---|---|---|---|
| Toolchain sayısı | 1 (cargo) | 2 (cargo + zig) | 2 (cargo + cc/MSVC/clang) |
| 3 OS cross‑compile | `tauri-action` ile hazır | Zig iyi cross‑compile eder ama build.rs entegrasyonu el işi | MSVC/clang/gcc farkları, el işi |
| Bellek güvenliği | Var (unsafe sadece OS çağrılarında) | Kısmen; FFI sınırı unsafe | Yok; FFI sınırı unsafe |
| Paralellik | rayon, crossbeam — olgun | Manuel thread havuzu | Manuel (pthread / Win32) |
| OS API erişimi | `windows`, `libc`, `rustix` crate'leri: `getattrlistbulk`, `statx`, `FindFirstFileExW`, NTFS MFT okuma mümkün | Doğrudan C ABI, iyi | Doğrudan, iyi |
| Ham performans | C ile aynı sınıf; darboğaz CPU değil **syscall/IO** | Aynı sınıf | Aynı sınıf |
| Katkıcı havuzu (OSS) | Tauri kullanıcıları zaten Rust bilir | Küçük | Büyük ama güvenlik riski |
| Dil olgunluğu | Stabil (edition 2024) | 1.0 öncesi, sürümler arası kırılma | Stabil |

**Neden darboğaz dil değil:** Disk taramasında zaman, dosya sistemi metadata çağrılarında (`readdir`+`stat`) geçer. Hızı belirleyen şey (1) çağrı sayısını azaltan toplu API'ler (macOS `getattrlistbulk`, Windows `FindFirstFileExW + FIND_FIRST_EX_LARGE_FETCH`, NTFS MFT'yi doğrudan okumak), (2) iyi bir paralel iş dağıtımıdır. Bunların hepsi Rust'tan doğrudan çağrılabilir; Zig/C eklemek hız kazandırmaz, sadece ikinci bir toolchain ve unsafe bir FFI sınırı ekler.

**Öneri: Sadece Rust (A).** Zig'i yalnızca ileride ölçülmüş, Rust'ta çözülemeyen bir darboğaz çıkarsa yeniden değerlendiririz (pratikte beklemiyorum).

> Bu karar `tasks/decisions.md` içine **ADR‑0001: Backend dili Rust** olarak işlenecek. Senin onayını bekliyor.

---

## 2. Kapsam ve özellikler

### 2.1 Çekirdek (v1.0)
1. **Disk/birim özeti** — bağlı tüm birimler: kapasite, kullanılan, boş, dosya sistemi tipi (APFS, NTFS, ext4, btrfs…), çıkarılabilir mi, ağ mı.
2. **Tarama** — tam disk ya da seçilen klasör; canlı ilerleme (dosya sayısı, bayt, hız, geçerli yol), iptal; sonuçlar tarama bitmeden kademeli görünür.
3. **Görünümler**
   - **Treemap** (squarified, canvas/WebGL) — tıklayınca içine girer (drill‑down), breadcrumb ile geri.
   - **Sunburst** (halka grafik) — alternatif görünüm.
   - **Ağaç tablo** — sıralanabilir: boyut, diskte kaplanan boyut, dosya sayısı, % ebeveyn, son değişiklik.
   - **Dosya türü dağılımı** — uzantı ve kategori (video, görsel, arşiv, kod, uygulama, sistem…).
   - **En büyük N dosya / klasör** listesi.
4. **Aksiyonlar** — Finder/Explorer/dosya yöneticisinde göster, aç, yolu kopyala, **çöpe taşı (varsayılan)**, kalıcı sil (ayardan açılır + yazarak onay), çoklu seçim "sepeti" ve toplam kazanılacak alan.
5. **Arama ve filtre** — ada/uzantıya/boyuta/tarihe göre.
6. **Tema & dil** — açık/koyu, TR/EN (i18n baştan).

### 2.2 Analiz (v1.x)
- **Kopya dosyalar** — boyut → ilk/son 64 KB hash → tam hash (BLAKE3/xxh3) kademeli eleme.
- **Eski/dokunulmamış dosyalar**, **boş klasörler**, **büyük indirme klasörü** uyarıları.
- **Akıllı temizlik kural paketleri** (veri dosyası olarak, TOML): tarayıcı önbellekleri, npm/pnpm/yarn/cargo/gradle/pip önbellekleri, Xcode DerivedData & simülatörler, Docker imajları, `node_modules`/`target`/`.venv` bulucu, log'lar, temp klasörleri, çöp kutuları. Her kural: yol kalıbı, OS, risk seviyesi, açıklama. Topluluk katkısına açık.
- **Anlık görüntü (snapshot) ve fark** — taramayı kaydet, sonradan "ne büyüdü?" karşılaştırması.
- **Dışa aktarma** — CSV / JSON.

### 2.3 Sonra (v2)
- Dosya sistemi izleme (`notify`) ile canlı güncelleme, menü çubuğu/tepsi widget'ı, zamanlanmış rapor.
- **`cleany` CLI** — aynı çekirdek crate'i kullanan terminal sürümü (OSS için çok değerli; CI'da test kolaylığı da sağlar). *Not: CLI'yi M1'de iç araç olarak zaten yazacağız.*

### 2.4 Karar: tıklama = içine gir (ADR-0004)
- Tek tık bir klasörün **içine girer** (drill‑down); breadcrumb ile geri dönülür. Dosyaya tıklamak onu seçer ve detayını gösterir.
- Silme **asla** tek tıkla olmaz: seç → sepete ekle → "Çöpe taşı" onayı (M4).

---

## 3. Mimari

```
Cleany/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ cleany-core/            # model (arena ağaç), tarayıcı, agregasyon, kopya bulucu, kurallar
│  ├─ cleany-platform/        # OS'a özel hızlı yollar + izinler + çöp/reveal
│  └─ cleany-cli/             # ince CLI (test & benchmark aracı)
├─ src-tauri/                 # Tauri uygulaması: komutlar, olaylar, capability'ler
├─ ui/                        # frontend (Vite + React + TS) — design kit burada
├─ rules/                     # temizlik kural paketleri (TOML)
├─ docs/                      # plan, ADR'ler, mimari
└─ .github/workflows/         # CI + release matrisi
```

### 3.1 Veri modeli (Rust tarafında kalır)
- **Arena ağaç**: `Vec<Node>`; `u32` indeksler (`parent`, `first_child`, `next_sibling`), `size_logical: u64`, `size_alloc: u64`, `file_count`, `mtime`, `kind/flags`, ad için string havuzuna offset. Hedef ~40 bayt/düğüm → **1M dosya ≈ 40–60 MB RAM**.
- Klasör boyutları tarama sonrasında tek geçişte alt→üst toplanır; tarama sırasında yaklaşık değerler canlı yayınlanır.

### 3.2 Tarayıcı
- İş çalan (work‑stealing) paralel dizin gezinme (rayon veya `jwalk` benzeri özel havuz).
- Platform hızlı yolları (cleany-platform):
  - **macOS**: `getattrlistbulk` (tek çağrıda dizin + metadata).
  - **Linux**: `getdents64` + `statx` (`rustix` ile).
  - **Windows**: `FindFirstFileExW(FindExInfoBasic, LARGE_FETCH)`; yönetici yetkisi varsa opsiyonel **NTFS MFT doğrudan okuma** (WizTree tarzı, ~10–50x hız).
- Genel geri dönüş yolu: `std::fs` / `walkdir` — her platformda doğruluk referansı.

### 3.3 Doğruluk kuralları (sık yapılan hatalar)
- **Mantıksal boyut** ve **diskte kaplanan boyut** ayrı raporlanır (sparse dosyalar, sıkıştırma, küçük dosya blok yuvarlaması).
- **Hardlink**'ler `(device, inode)` / Windows `FileId` ile bir kez sayılır.
- **Symlink, junction, reparse point** takip edilmez; **mount sınırı** varsayılan olarak geçilmez (ayarlanabilir).
- **APFS klonları, snapshot'lar, purgeable alan**: toplamın `df`/Finder ile neden tutmadığı kullanıcıya açıklanır ("Sistem/Diğer" kalemi).
- **iCloud / OneDrive / Dropbox yer tutucu** dosyalar asla okunmaz/indirilmez (yalnızca metadata; yerel olmayan boyut ayrı gösterilir).
- İzin reddedilen yollar sayılır ve listelenir; tarama durmaz.
- Windows uzun yol (`\\?\`) desteği; Unicode/geçersiz UTF‑8 isimler kayıpsız (`OsString`).

### 3.4 IPC (Rust ↔ UI)
UI'a asla milyonlarca düğüm gönderilmez. UI sadece görünen dilimi ister:
- `list_volumes()`
- `start_scan(path, opts) -> ScanId` ; ilerleme `tauri::ipc::Channel` ile **≤10 Hz** throttle
- `cancel_scan(id)`
- `get_children(node, sort, offset, limit)`
- `get_treemap(node, depth, min_area)` → yalnızca çizilecek kadar düğüm
- `get_breakdown(node, by: ext|category)`, `top_files(node, n)`, `search(query)`
- `reveal(node)`, `open(node)`, `trash(nodes) -> Report`, `delete(nodes, confirm_token)`
- Tipler `tauri-specta` (aday) ile TypeScript'e otomatik üretilir.

### 3.5 Frontend
- **Vite + React + TypeScript + Tailwind + shadcn/ui** (design kit örnekleri React/Tailwind varsayıyor; shadcn MCP'si de elimizde).
- Treemap/sunburst **canvas** üzerinde (gerekirse WebGL); yerleşim hesabı `d3-hierarchy` ya da Rust tarafında. DOM ile çizilmez.
- Durum: Zustand veya TanStack Query (komut önbelleği için).
- **Performans testi özellikle Linux WebKitGTK'de** yapılır (en zayıf webview).

---

## 4. İzinler ve platform notları
- **macOS**: Tam tarama için **Full Disk Access** gerekir → uygulama ilk açılışta yönlendirir. Sandbox'lı olamaz ⇒ **Mac App Store yok**; dağıtım **GitHub Releases üzerinden DMG** (universal: arm64 + x64), opsiyonel Homebrew cask. Gatekeeper uyarısı çıkmaması için Developer ID imzası + notarization.
- **Windows**: Normal tarama yetkisiz; MFT hızlı yolu için yönetici gerekir → "Hızlı mod için yönetici olarak yeniden başlat" seçeneği. Kod imzalama (SignPath OSS ücretsiz programı ya da Azure Trusted Signing).
- **Linux**: AppImage, .deb, .rpm; AUR. **Flatpak** sandbox'ı görünürlüğü kısıtlar (`--filesystem=host` gerekir) → Flathub sonraya.

## 5. Güvenlik
- Frontend'e `fs` plugin'i **açılmaz**; tüm dosya işlemleri doğrulamalı Rust komutlarıyla yapılır. Tauri capability'leri en dar kapsamda.
- **Korumalı yollar listesi**: `/`, `/System`, `/usr`, `C:\Windows`, `C:\Program Files`, ev dizini kökü vb. — silme engellenir.
- Varsayılan **çöpe taşı** (`trash` crate, aday); kalıcı silme ayardan açılır + yazarak onay.
- Her silme işlemi yerel bir log'a yazılır (ne, ne zaman, nereye).
- Sıkı CSP; uzak içerik yok; telemetri yok (varsa opt‑in).

## 6. Test ve kalite
- **Birim/entegrasyon**: tempdir fikstürleri — hardlink, symlink döngüsü, sparse dosya, izinsiz klasör, Unicode isimler, derin ağaç.
- **Referans karşılaştırma**: `du -sk` / `Get-ChildItem` ile çıktı karşılaştıran CI testi.
- **Benchmark**: `criterion` + sentetik 1M dosyalık ağaç; sürüm başına regresyon takibi.
- **E2E**: `tauri-driver` + WebdriverIO (Linux/Windows; macOS'ta WKWebView sürücüsü yok → manuel/Playwright UI testi).
- CI: GitHub Actions matrisi (macos-latest arm64 + x64, windows-latest, ubuntu-22.04): `fmt`, `clippy -D warnings`, test, UI lint/typecheck.

## 7. Release
- `tauri-action` ile üç platform paketleri doğrudan **GitHub Releases**'e: macOS `.dmg`, Windows `.msi`/`.exe` (NSIS), Linux `.AppImage`/`.deb`/`.rpm`. Mac App Store / Microsoft Store hedeflenmiyor.
- **Updater** plugin'i (imzalı güncellemeler).
- Dağıtım: Homebrew cask, winget, Scoop, AUR, (sonra) Flathub.
- Sürümleme: SemVer + conventional commits → otomatik CHANGELOG (release-please).

## 8. Claude Code Kit + Claude Design Kit kurulumu
İki kit **aynı klasörü paylaşamıyor** (aynı `CLAUDE.md` ve hook adlarını kullanıyorlar; installer durur). Dokümanların önerdiği çözüm alt klasör kurulumu. Tauri yapısına uyan yerleşim:

| Klasör | Kit | Kapsam |
|---|---|---|
| repo kökü | **claude-code-kit** `--template rust` | Rust workspace, src-tauri, CI, release |
| `ui/` | **claude-design-kit** | DESIGN_MAP, token'lar, bileşenler, motion |

- Hook'lar yalnızca oturumun açıldığı klasörde çalışır → backend işi kökten, UI işi `ui/` içinden açılan oturumla yapılır. Bu kural `CLAUDE.project.md`'ye yazılacak.
- Code kit `rust` şablonu sadece Cargo kuralları getiriyor; frontend kuralları `ui/`'deki design kit + `ui/CLAUDE.project.md` ile kapanacak.
- Kurulum sırası (M0):
  1. İlk commit (README, LICENSE, .gitignore) **kit kurulmadan önce** `main`'e push'lanır (code kit'in `branch-protect` hook'u sonrasında `main`'e push'u engelleyecek; sonraki her iş branch + PR).
  2. `npx @tansuasici/claude-code-kit init --template rust --diff` → kontrol → `init`.
  3. `ui/` iskeleti (Vite) → `npx @tansuasici/claude-design-kit init` (ui/ içinde) → `/design-brief` → `/direction` (onayına sunulur).
  4. `protect-changes` hook'u bağımlılık/build config değişikliklerini onaysız engeller → iskelet kurulumu senin onayınla (`CLAUDE_APPROVED=1`) yapılır.

## 9. Yol haritası (milestone'lar)

Süreler tek geliştirici + Claude, yarı zamanlı tempo için kaba tahmindir.

| # | Milestone | Süre | Kabul kriteri |
|---|---|---|---|
| **M0** | Altyapı | 1 hf | Repo, kitler, Cargo workspace, Tauri+Vite iskeleti 3 OS'ta açılıyor, CI yeşil, ADR‑0001, DESIGN_MAP yön onaylı |
| **M1** | Çekirdek tarayıcı + CLI | 2–3 hf | `cleany-cli scan <yol>` 3 OS'ta doğru toplam (du ile ±%1), hardlink/symlink/sparse testleri geçer, 1M dosya < 150 MB RAM |
| **M2** | Uygulama kabuğu | 2 hf | Birim listesi, tarama başlat/iptal, canlı ilerleme, ağaç tablo, drill‑down, breadcrumb |
| **M3** | Görselleştirme | 2 hf | Canvas treemap + sunburst, hover bilgi, tıkla‑içine‑gir, 60 fps (WebKitGTK dahil) |
| **M4** | Aksiyonlar & güvenlik | 2 hf | Reveal/aç/kopyala, çöpe taşı, seçim sepeti, korumalı yollar, silme logu, onay akışları |
| **M5** | Analiz | 2–3 hf | Tür dağılımı, top‑N, eski dosyalar, boş klasörler, kopya bulucu, arama/filtre |
| **M6** | Akıllı temizlik | 2 hf | TOML kural paketleri (3 OS), risk etiketleri, kural katkı rehberi |
| **M7** | Hız & geçmiş | 2 hf | macOS getattrlistbulk, Windows MFT modu, snapshot kaydet/karşılaştır, CSV/JSON dışa aktarma |
| **M8** | Yayın v1.0 | 2 hf | İmzalı/notarize paketler, updater, Homebrew/winget/AUR, docs sitesi, CONTRIBUTING, v1.0.0 |

**Toplam:** ~17–21 hafta. M1 ile M2'nin UI kısmı paralel ilerleyebilir (UI sahte veriyle).

## 10. Riskler
| Risk | Etki | Önlem |
|---|---|---|
| WebKitGTK'de canvas performansı | Linux'ta takılma | Düğüm sayısı sınırlı dilimler, erken benchmark (M3) |
| macOS izin akışı kafa karıştırıcı | Eksik tarama | Onboarding ekranı + "erişilemeyen yollar" raporu |
| Yanlış silme | Veri kaybı, güven kaybı | Çöp varsayılan, korumalı yollar, onay, log |
| Kod imzalama maliyeti | SmartScreen/Gatekeeper uyarıları | Apple Developer ($99/yıl), SignPath OSS başvurusu |
| MFT okuma karmaşıklığı | Gecikme | M7'ye ertelendi; genel yol her zaman çalışır |

## 11. Karar bekleyen sorular
1. ~~**Dil**~~ → sadece Rust (ADR-0001).
2. ~~**"Tıklayınca giden"**~~ → içine gir (ADR-0004).
3. ~~**Lisans**~~ → MIT OR Apache‑2.0 (ADR-0002).
4. **Frontend**: React + Tailwind + shadcn uygun mu?
5. **Minimum OS**: macOS 12+, Windows 10+, Ubuntu 22.04+ öneriliyor.
6. **Apple Developer hesabı** var mı (notarization için)?
