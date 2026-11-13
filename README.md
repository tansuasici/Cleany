# Cleany

**Diskinde yer nereye gitti?** Cleany, macOS, Windows ve Linux için açık kaynak bir disk alanı analiz ve temizlik aracıdır. Hızlı tarar, alanı treemap / sunburst ile gösterir, gereksiz dosyaları güvenle (önce çöpe) temizlemeni sağlar.

> 🚧 Erken geliştirme aşamasında. Yol haritası: [docs/PLAN.md](docs/PLAN.md)

## Özellikler (planlanan)
- Tüm birimler için kapasite / kullanım özeti
- Paralel, hızlı tarama (macOS `getattrlistbulk`, Windows NTFS MFT hızlı modu)
- Treemap, sunburst ve sıralanabilir ağaç tablo; tıkla-içine-gir
- Dosya türü dağılımı, en büyük dosyalar, kopya dosyalar, eski dosyalar
- Geliştirici önbellekleri ve sistem çöpü için akıllı temizlik kuralları
- Güvenli silme: varsayılan çöp kutusu, korumalı yollar, silme kaydı

## İndirme
Sürümler [GitHub Releases](https://github.com/tansuasici/Cleany/releases) üzerinden yayınlanacak:

| Platform | Paket |
|---|---|
| macOS (Apple Silicon + Intel) | `.dmg` |
| Windows 10/11 | `.msi`, `.exe` (kurulum) |
| Linux | `.AppImage`, `.deb`, `.rpm` |

## Teknoloji
[Tauri 2](https://tauri.app) · Rust (tarama çekirdeği) · React + TypeScript + Vite (arayüz)

## Lisans
Cleany, [MIT](LICENSE-MIT) veya [Apache-2.0](LICENSE-APACHE) lisanslarından biri altında, tercihinize göre kullanılabilir.
