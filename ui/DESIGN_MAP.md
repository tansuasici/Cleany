# DESIGN_MAP.md

> The single source of truth for this product's design direction. Every color,
> typeface, spacing value, icon and animation should trace back to a line here.
> Fill it with `/design-brief` (Product, Competitors, inventory) and
> `/direction` (Character → Tokens). Replace every `<…>`; session-start.sh
> treats `<` as "still the template".
>
> Changing Character, World, Signature or the palette needs the user's approval.

## Product
Cleany, diski dolan sıradan bir kullanıcıya "yer nereye gitti?" sorusunun cevabını gösterir. Ekranların tek işi kullanıcıyı en çok yer kaplayan şeye götürmek ve orayı güvenle (önce çöpe atarak) boşaltmasını sağlamak. M2'de bu akış şöyle: birim seç → tara → en büyük klasöre in.

- **Kind:** app screen (worked in). Masaüstü uygulama penceresi, Tauri 2 webview.
- **Audience:** genel kullanıcılar. Yol, bayt, inode gibi teknik ayrıntılar arka planda kalır, istenince görünür. Geliştirici önbellekleri (node_modules, DerivedData, Docker) özellik olarak var, ama dil ve ton geliştiriciye göre ayarlanmaz.
- **Existing brand:** yok, sıfırdan başlanıyor. `ui/public/favicon.svg` (lacivert zemin, gök mavisi / mor / yeşil treemap blokları) yer tutucu, bağlayıcı değil. Logo sonra.
- **Constraints:** React 19 + TypeScript + Tailwind v4 (`ui/src/index.css`). M2'de bileşen kütüphanesi yok (shadcn eklenmiyor). Açık ve koyu tema. TR/EN, RTL yok. macOS / Windows / Linux. En zayıf webview Linux WebKitGTK, efekt bütçesi ona göre belirlenir. Erişilebilirlik hedefi: WCAG 2.1 AA (2026-11-14 onaylandı).
- **Assets that exist:** yalnızca yer tutucu favicon ve `src-tauri/icons/*` (aynı yer tutucu). Ekran görüntüsü, fotoğraf, illüstrasyon yok.

## Character
Sakin, hesap verir gibi net, dürüst. (Yön A "Döküm fişi", 2026-11-14 onaylandı.)
<!-- Don't use: clean, modern, sleek, minimal, premium, intuitive, seamless. slop-scan.sh flags them here. -->

## World
Esnaf defteri ve makbuz: ince kâğıt, hizalı rakamlar, alt toplam çizgisi, kopya kalemi mavisi. Uygulama ekranı olduğu için zemin ve paneller nötr kalır. Dünya vurgu renginde, rakamların dizilişinde ve iki anda görünür.

Subject inventory (8–12 things only this product has; ham malzeme, henüz bir dünya değil):
- **Soru:** "Diskinde yer nereye gitti?" (README'nin ilk cümlesi). Ürün bir cevap verir, alarm çalmaz.
- **Birim:** "Macintosh HD", `C:\`, `/Volumes/USB`. Kapasite, kullanılan, boş; çıkarılabilir olup olmadığı.
- **İki boyut:** bir dosyanın mantıksal boyutu ile diskte gerçekten kapladığı (allocated) boyut farklıdır. Cleany diskte kaplananı esas alır.
- **Dürüst sayım:** hardlink'ler bir kez sayılır. iCloud/OneDrive yer tutucuları diskte yer kaplamaz ve öyle işaretlenir.
- **İç içe klasörler:** tek tıkla içine girilir, breadcrumb ile geri çıkılır (ADR-0004). Kökten yaprağa giden bir yol.
- **Orantılı bloklar:** treemap ve sunburst; blok alanı boyutu gösterir (M3).
- **Canlı tarama sayaçları:** dosya, klasör, bayt, geçen süre. Saniyede en fazla yaklaşık 10 güncelleme; tarama her an iptal edilebilir.
- **Önce çöp:** silme asla tek tıkla olmaz. Seç → sepete ekle → kazanılacak toplam alanı gör → "Çöpe taşı" onayı. Silme kaydı tutulur.
- **Korumalı yollar ve okunamayanlar:** sistem yolları korunur. Erişilemeyen girdiler sayılır ve raporlanır (ilk 100 tanesi ve toplam sayı).
- **İzin adımı:** macOS'ta Full Disk Access gerekir; ilk açılışta kullanıcı oraya yönlendirilir.
- **Dosya türleri:** video, görsel, arşiv, kod, uygulama, sistem (M5).
- **"Ne büyüdü?":** taramayı kaydedip sonradan karşılaştırma (M7). Açık kaynak, üç platform.

## Moments
1. **Birim kartının doluluk çubuğu.** Tarama bitince çubuk, en büyük üst klasörlere göre bölümlere ayrılır.
2. **Tarama sonu dökümü.** Üst düzeydeki kalemler (en büyük 12'si, kalanı "Diğer N kalem"), her birinin payı ve en büyüğe göre ölçekli ince bir çubuğu; klasörler istenince yerinde bir seviye açılır. Toplam yalnızca bir kez, alt toplam çizgisinin altında yazılır. "Sayılmayanlar" dipnotu yalnızca sayılmayan bir şey varsa görünür: hardlink'ler bir kez sayıldı, iCloud/OneDrive yer tutucuları diskte yer kaplamıyor, N öğe okunamadı. (Kalem sayısı ve pay sütunu 2026-11-14 onaylandı.)

Bunların dışındaki her şey sessiz kalır.

## Signature
Adla boyutu birbirine bağlayan **noktalı çizgi**: `Belgeler ··········· 48,2 GB`. Tarama dökümünde, sepette ("Kazanılacak alan ····· 12,4 GB") ve breadcrumb toplamında kullanılır. Yoğun ağaç tabloda kullanılmaz; orada yalnızca sağa hizalı, eşit genişlikli rakamlar olur.

**Idea sentence:** Bu ekranda tarama sonucu, diskin döküm fişine dönüşür: kalemler, alt toplam, dürüst dipnotlar.

**Competitor test (2026-11-14):** DaisyDisk (sunburst ve renk gösterisi), WizTree / WinDirStat (yoğun araç tablosu), GrandPerspective / Baobab (görsel harita) ve CleanMyMac (satış ve uyarı tonu) bu cümleye de karakter satırına da uymuyor.

## Tokens
| Token | Value | Why (which line above) |
|---|---|---|
| Neutral ramp | Açık (light warm): page `#f7f4ee`, card `#fdfbf7`, raised `#efe9df`, text `#1f1b16` / `#5e554a` / `#756c60`. Koyu (dark warm): deepest `#12110f`, page `#161513`, card `#1c1a18`, raised `#262421`, text `#efe9de` / `#aaa398` / `#8a847a`. Hairline: birincil metin %8 opaklıkta. Soluk metin (`#756c60` / `#8a847a`) yalnızca page ve card üstünde kullanılır, raised üstünde değil. | World: kâğıt; uygulama ekranı olduğu için hafif tonlu |
| Accent | Kopya kalemi mavisi: açık `#2547b8`, koyu `#8ea6ff` (kontrast ölçüldü, 2026-11-14: açık 7,16:1 page, 7,60:1 card; koyu 7,86:1 page, 7,48:1 card). Yalnızca seçim, birincil eylem ve dökümdeki en büyük kalemin çubuğu (2026-11-14 onaylandı; toplam rakamı vurgu almaz). | World: kopya kalemi |
| Semantic | Pozitif `#16a34a` / `#4cc38a`, negatif `#dc2626` / `#e5484d`; negatif yalnızca gerçek hata için kullanılır, "disk dolu" için değil. Negatif metin AA'yı her zeminde geçmez (ölçülen 3,95–4,67:1); metin rengi olarak değil, kenar/işaret olarak kullanılır | Don'ts: korkutma yok |
| Type | Sistem fontu (`system-ui`: SF / Segoe UI / Cantarell). Rakamlar `tabular-nums`, boyut sütunları `ui-monospace`. Font paketlenmez. | Character: net, yerli; "fiş" hissini rakam hizası taşır. Çevrimdışı uygulama, CSP dış kaynağa izin vermez |
| Type scale | 12/16 meta · 13/20 ikincil · 14/20 gövde · 15/22 vurgulu gövde · 20/28 sayfa başlığı (500) · 28/1 anahtar rakam (500, tabular). Başlık rakamdan küçüktür: ekranın cevabı rakamdır (2026-11-14 onaylandı). Ağırlık 400/500, nadiren 600. | |
| Spacing | 4px base: 4 8 12 16 20 24 32 40 48 64 80 96 128. Ağaç tablo satırı 32. | |
| Radius | Etiket 4 · satır, girdi, düğme 6 · kart 8 · panel 10 (iç içe olanlar eş merkezli) | World: kâğıt, küçük köşe |
| Leader | Noktalı çizgi: `border-bottom: 1px dotted` (muted metin rengi), taban çizgisine hizalı | Signature |
| Motion | ease-out `cubic-bezier(0.23, 1, 0.32, 1)`, UI < 300ms. Canlı sayaçlar animasyonsuz güncellenir. | Character: sakin |
| Icons | M2: 16px, 1.5 stroke, elle çizilmiş inline SVG (birim, çıkarılabilir, geri, iptal). İkon seti eklenmez. | Bağımlılık yok; ihtiyaç büyürse onayla lucide |

## Competitors
<!-- Tek satırlık özetler genel bilgiden yazıldı, ekran görüntüleriyle doğrulanmadı. -->
- DaisyDisk (macOS): koyu zeminde büyük, renkli bir sunburst. Seçilenler sürüklenip bir "toplama" alanına bırakılır.
- WizTree / WinDirStat (Windows): sıkışık bir ağaç tablo, altında renkli düz bir treemap. Hız ve yoğunluk odaklı, araç görünümlü.
- GrandPerspective / Baobab (açık kaynak, macOS / GNOME): treemap ya da halka grafik. Sistem arayüzüne gömülü, sade ve teknik.
- CleanMyMac (ticari): parlak, pazarlama dilli; "tek tuşla temizle" ve uyarı odaklı akışlar.

## Don'ts
<Corrections that should hold in future sessions. Add one each time the user says "this isn't us".>
- Korkutma ve satış tonu yok: "diskin tehlikede!" alarmları, kırmızı uyarı yağmuru, upsell yok (2026-11-14, kullanıcı).
- Renkli gradyan ve glow efekti yok (2026-11-14, kullanıcı).
