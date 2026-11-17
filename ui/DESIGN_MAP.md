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
- **Existing brand:** "Halka" işareti: C ve üç noktalı bir sıra. Genel logo, yuvarlak köşeli mavi rozet (#2547b8 zemin, #f7f4ee işaret, köşe yarıçapı kenarın %22,5'i): navbar, favicon ve uygulama ikonu aynı rozeti kullanır. Rozetsiz işaret (`assets/logo*.svg`) yalnızca rozetin sığmadığı tek renk yerler için.
- **Constraints:** React 19 + TypeScript + Tailwind v4 (`ui/src/index.css`). M2'de bileşen kütüphanesi yok (shadcn eklenmiyor). Açık ve koyu tema. TR/EN, RTL yok. macOS / Windows / Linux. En zayıf webview Linux WebKitGTK, efekt bütçesi ona göre belirlenir. Erişilebilirlik hedefi: WCAG 2.1 AA (2026-11-14 onaylandı).
- **Assets that exist:** `assets/` (logo ve ikon SVG'leri), `ui/public/favicon.svg`, `src-tauri/icons/*` (macOS plakalı, Windows tam dolu). Ekran görüntüsü, fotoğraf, illüstrasyon yok.

## Character
Sakin, yumuşak köşeli, hesap verir gibi net, dürüst. (Yön A "Döküm fişi", 2026-11-14 onaylandı; 2026-11-17 yumuşatıldı: yumuşak köşe, daha az çizgi, küçük hareketler.)
<!-- Don't use: clean, modern, sleek, minimal, premium, intuitive, seamless. slop-scan.sh flags them here. -->

## World
Esnaf defteri ve makbuz, ama elde yumuşamış kâğıt: yuvarlak köşe, hizalı rakamlar, tek ince alt toplam çizgisi, kopya kalemi mavisi. Uygulama ekranı olduğu için zemin ve paneller nötr kalır. Dünya vurgu renginde, rakamların dizilişinde ve iki anda görünür.

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
2. **Tarama sonu dökümü.** Üst düzeydeki kalemler (en büyük 12'si, kalanı "Diğer N kalem"; %1'in altındakiler ayrıca katlanmaz, 2026-11-15), her birinin payı ve en büyüğe göre ölçekli ince bir çubuğu; klasörler istenince yerinde bir seviye açılır. Toplam yalnızca bir kez, alt toplam çizgisinin altında yazılır. "Sayılmayanlar" dipnotu yalnızca sayılmayan bir şey varsa görünür: hardlink'ler bir kez sayıldı, iCloud/OneDrive yer tutucuları diskte yer kaplamıyor, N öğe okunamadı. (Kalem sayısı ve pay sütunu 2026-11-14 onaylandı.)

Bunların dışındaki her şey sessiz kalır.

## Signature
Adla boyutu birbirine bağlayan **noktalı çizgi**: `Belgeler ··········· 48,2 GB`. Yalnızca tarama dökümünde kullanılır: kalemler, "Diğer N kalem" ve Toplam (2026-11-17). Sayaçlarda, birim kartında, sepette ve diyalog listelerinde ad solda, rakam sağda durur, arası boş kalır. Toplamın üstünde fişin **alt toplam çizgisi** durur: tek 1px `edge` çizgisi (`rule-total`; çift çizgi 2026-11-17'de tek çizgiye indi).

**Idea sentence:** Bu ekranda tarama sonucu, diskin döküm fişine dönüşür: kalemler, alt toplam, dürüst dipnotlar.

**Competitor test (2026-11-14):** DaisyDisk (sunburst ve renk gösterisi), WizTree / WinDirStat (yoğun araç tablosu), GrandPerspective / Baobab (görsel harita) ve CleanMyMac (satış ve uyarı tonu) bu cümleye de karakter satırına da uymuyor.

## Tokens
| Token | Value | Why (which line above) |
|---|---|---|
| Neutral ramp | Açık (light warm): deepest `#e9e3d8`, page `#f1ece3`, card `#fdfbf7`, raised `#efe9df`, text `#1f1b16` / `#5e554a` / `#6b6257`. Koyu (dark warm): deepest `#0d0c0b`, page `#12110f`, card `#1f1d1a`, raised `#2a2825`, text `#efe9de` / `#aaa398` / `#948e83`. Kâğıt (card) zeminden belirgin ayrılır; fiş zeminin üstünde durur (2026-11-16 onaylandı). Kart ve panel kenarı (`card-edge`): açıkta yok, koyuda birincil metin %8 (2026-11-17). Hairline: birincil metin %12; yalnızca tablo başlığının altında ve kart içindeki ayraçlarda. Kontrol kenarı (`edge`) %22; 3:1 gereken sınırda (`edge-strong`) açıkta %50, koyuda %40. Soluk metin yalnızca page ve card üstünde kullanılır, raised üstünde değil. | World: kâğıt; fiş zeminden ayrı bir yaprak |
| Elevation | Üç basamak, hepsi durağan, hiçbiri animasyon almaz (2026-11-17). Kart: açıkta `0 1px 2px rgb(31 27 22 / .06)`. Yüzen (ipucu, harita üstü kart, menü): açıkta `0 6px 16px -6px rgb(31 27 22 / .18)` + hairline kenar. Diyalog: açıkta `0 16px 40px -12px rgb(31 27 22 / .22)` + hairline kenar. Koyuda gölge yok; ayrımı kenar verir (kart %8, yüzen ve diyalog %16). | World: masaya bırakılmış kâğıtlar |
| Viz ramp | Harita ve halka: boyut sırasına göre ink opaklığı, açık %34/26/18/12/7, koyu %26/20/14/9/5; "Diğer" %4 (koyu %3) + taralı. Etiketler doğrudan blokta, yalnızca ink. En büyük blok vurgu alır (2026-11-15 kuralı sürer). (2026-11-16 onaylandı) | World: kopya kâğıdı karbonu; renk değil ton |
| Accent | Kopya kalemi mavisi: açık `#2547b8`, koyu `#8ea6ff` (kontrast ölçüldü, 2026-11-14: açık 7,16:1 page, 7,60:1 card; koyu 7,86:1 page, 7,48:1 card). Yalnızca seçim, birincil eylem ve dökümdeki en büyük kalemin çubuğu (2026-11-14 onaylandı; toplam rakamı vurgu almaz). Birim çubuğunun en büyük bölümü vurgu almaz (2026-11-15, reddedildi). | World: kopya kalemi |
| Semantic | Pozitif `#16a34a` / `#4cc38a`, negatif `#dc2626` / `#e5484d`; negatif yalnızca gerçek hata için kullanılır, "disk dolu" için değil. Negatif metin AA'yı her zeminde geçmez (ölçülen 3,95–4,67:1); metin rengi olarak değil, kenar/işaret olarak kullanılır | Don'ts: korkutma yok |
| Type | Sistem fontu (`system-ui`: SF / Segoe UI / Cantarell). Rakamlar `tabular-nums`; boyutlar da sistem fontuyla (2026-11-17). Mono yalnızca yol ve ham hata metninde. Font paketlenmez. | Character: net, yerli; "fiş" hissini rakam hizası taşır. Çevrimdışı uygulama, CSP dış kaynağa izin vermez |
| Type scale | 12/16 meta · 13/20 ikincil · 14/20 gövde · 15/22 vurgulu gövde · 20/28 sayfa başlığı (500) · 28/1 anahtar rakam (500, tabular) · 40/44 `display` (500, ekran başına en fazla bir kez: boş alan, bulunan bayt; 2026-11-16 onaylandı). Başlık rakamdan küçüktür: ekranın cevabı rakamdır (2026-11-14 onaylandı). Ağırlık 400/500, nadiren 600. Birim (GB) 0,6em, ink-2 (`unit`). Yüzde, dosya sayısı ve tarih sans `tabular-nums` (2026-11-16 onaylandı). | |
| Spacing | 4px base: 2 4 8 12 16 20 24 32 40 48 64 80 96 128. 2 yalnızca çubuk aralığı ve etiket iç boşluğu gibi ince ayarlar için (2026-11-15 onaylandı). Ağaç tablo satırı 32. | |
| Radius | Etiket 6 · satır, girdi, düğme 8 · kart 12 · panel 16 · ince çubuk tam yuvarlak (iç içe olanlar eş merkezli: panel 16 = iç boşluk 8 + satır 8) (2026-11-17) | World: elde yumuşamış kâğıt |
| Leader | Noktalı çizgi: 2px nokta, 6px aralıkla, `radial-gradient` ile (soluk metin rengi, ink-3), taban çizgisine hizalı. Yalnızca dökümde (2026-11-17). `1px dotted` webview'e göre değişip silik kaldığı için bırakıldı (2026-11-15 onaylandı). Bu, Don'ts'taki "gradyan" yasağının konusu değil: renk geçişi yok, yalnızca nokta deseni. | Signature |
| Focus | Varsayılan: 2px vurgu halkası, 2px boşluk. Bileşik bileşenlerde (tablo, harita, halka) odak kapsayıcıdadır: kapsayıcı 1px vurgu kenarı alır, 2px halka etkin satır veya blokta çizilir; göz orada (2026-11-16). | Character: dürüst durum |
| Motion | ease-out `cubic-bezier(0.23, 1, 0.32, 1)`, UI < 300ms, yalnızca transform ve opacity (2026-11-17). Hareket alanlar: ekran değişimi (opacity + 6px yukarı, 180ms), görünüm değişimi (opacity, 120ms), dökümde açılan alt kalemler (opacity + 4px, 160ms), diyalog (opacity + scale .97, 180ms; çıkış anlık), ipucu (opacity + scale .97, 120ms; 400ms bekleme, klavye odağında anlık), düğmeye basış (scale .97, 100ms), birim doluluk çubuğu (scaleX, 240ms, bir kez), sepet çubuğu (opacity, 150ms). Hareket almayanlar: satır hover'ı, canlı sayaçlar, tarama çubuğu, eylem geri bildirimi, fareyi izleyen kartlar. `prefers-reduced-motion`: yalnızca opacity kalır. | Character: sakin |
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
- Satırda, kartta ya da mesajda soldan renkli şerit yok (`border-l-2`, inset accent çubuğu). Seçim yüzey adımıyla (`bg-raised`), vurgu tam hairline kenarlıkla verilir (2026-11-15, kullanıcı).
- Tıklanabilir her öğe el imleci (`cursor: pointer`) gösterir; devre dışı kontroller göstermez. Genel kural `index.css` base katmanında (2026-11-16, kullanıcı).
- Kartta karta ikinci kenarlık yok; ayrım yüzey adımıyla verilir. Gölge animasyon almaz, sanallaştırılmış satıra gölge konmaz. İskelet satırlarda pulse yok (2026-11-17, kullanıcı: "yumuşak hatlı").
- Arayüz metni yayın düzeyinde, laubali değil, kullanıcıya dönük (2026-11-16, kullanıcı). Uygulama: Türkçe açıklama ve ipuçlarında "siz" hitabı (seçin, yapabilirsiniz); düğme etiketleri kısa emir kipinde kalır (Tara, Yeniden tara, Çöpe taşı). Esprili başlık yok ("Yer nereye gitti?" kaldırıldı). Türkçe tırnak “…”. Çöp için genel sözcük ("çöp"), çünkü Windows da hedef.
- Odak halkası yok, hiçbir yerde: ne `outline` ne `ring` ne de odakta renklenen kenarlık. Genel kural `index.css` base katmanında `:focus, :focus-visible { outline: none }`. Klavye konumu yalnızca yüzey adımıyla (`bg-raised`) gösterilir. Bu, kitteki "görünür odak halkası" kuralının yerine geçer (2026-11-17, kullanıcı).
