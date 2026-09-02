<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — dört Catppuccin temasıyla ana arayüzü gösteren modern, çapraz platformlu bir Minecraft başlatıcı">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>Küçük, hızlı ve çevik bir Minecraft başlatıcı</strong><br>
  Çoklu instance yönetimi · Mod ve resource pack pazar yeri · Çapraz LAN çoklu oyuncu · Çoklu tema desteği</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md"><strong>Türkçe</strong></a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Bu nedir

Conic Launcher modern bir masaüstü Minecraft başlatıcısıdır: çekirdek mantığı Rust ile uygulanmış, arayüz Tauri 2 + Vue 3 ile oluşturulmuştur. Küçük kurulum boyutu, hızlı başlangıç ve düşük kaynak tüketimi ile karbon emisyonlarını azaltmaya bile yardımcı olur.

Instance oluşturma ve loader kurulumundan, mod aramaya ve arkadaşları birlikte oynamaya davet etmeye, oyunu başlatmaya ve oyun süresini takip etmeye kadar — tüm iş akışı tek bir uygulamada tamamlanabilir.

> Proje şu anda erken Alpha aşamasındadır. Özellikler ve arayüz hızla gelişmektedir — lütfen deneyin ve sorunları bildirin.

## Özellikler

### Çoklu instance yönetimi

Mod loader'a göre gruplanmış instance'ları sıralama, arama ve favoriler ile görüntüleyin. Her instance'ın kendi ayarları, özel arka planı, oyun süresi takibi ve son çalışma zamanı damgası bulunur.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Tüm ana loader'ları destekleme

Minecraft sürümünüzü ve loader sürümünüzü seçin — gerisini başlatıcı halleder. Vanilla, Forge, NeoForge, Fabric ve Quilt'un tamamı desteklenir, kurulum ilerlemesi gerçek zamanlı olarak gösterilir.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod ve resource pack pazar yeri

Dahili Modrinth ve CurseForge araması: proje ayrıntılarını ve bağımlılıkları görüntüleyin, çevrilmiş özetleri okuyun ve modları veya resource pack'ları tek tıklamayla instance'larınıza kurun. Yerel olarak kurulu modları doğrudan da yönetebilirsiniz.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus çapraz LAN çoklu oyuncu

Ekstra araç gerekmez: bir grup oluşturun, davet kodunu arkadaşlarınızla paylaşın ve aynı dünyaya birlikte katılın. Başlatıcı ağ ortamınızı (NAT türü) algılar ve iyileştirme önerileri sunar.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Çoklu giriş yöntemleri

Microsoft hesabı girişi (cihaz kodu akışı), çevrimdışı giriş ve herhangi bir Authlib-Injector / Yggdrasil harici kimlik doğrulama sunucusu desteklenir. Hesaplar 3D skin önizleme, pelerin gösterimi ve skin yükleme özelliği sunar.

<img width="840" height="533" alt="Ekran görüntüsü 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Rahatsızlık vermeyen başlatma deneyimi

Sisteminizdeki Java'yı otomatik olarak tarar veya uygun bir runtime'ı otomatik olarak indirir. Bellek otomatik olarak atanır (32-bit Java limit koruması dahil). Başlatma süreci tamamen görselleştirilir: oyun dosyalarını indirme, loader kurulumu, bağımlılıkları çözme ve sürecin başlamasını bekleme.

İleri düzey kullanıcılar da unutulmaz — JVM çöp toplayıcı, JVM argümanları, oyun argümanları, classpath, sarmalayıcı komut ve başlatma öncesi/sonrası komutların tamamı özelleştirilebilir.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Kayıtlar ve oyun verileri

Kayıt dünya haritalarını görüntüleyin, veri paketlerini ve resource pack'ları yönetin ve ekran görüntülerini görüntüleyin — hepsi başlatıcı içinde.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Arayüz ve kişiselleştirme

Dört Catppuccin teması (Mocha · Macchiato · Frappé · Latte), yüksek kontrastlı varyantlarla birlikte, sistem ayarlarınıza göre otomatik açık/koyu geçiş. Özel başlatıcı arka planları desteklenir. Yerel müzik çalma ve spektrum görselleştirme özellikli dahili müzik çalar.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### Ve daha fazlası

- Uygulama içi otomatik güncellemeler ve oyun sürümü güncelleme bildirimleri
- Ayarlanabilir bağlantı limitleri ve hız sınırlaması ile çoklu iş parçacıklı indirmeler; ayna sunucu ve sistem proxy desteği
- Küresel arama (`/`) ve komut paleti (`;`)
- Oyun süresi takibi ve etkinlik takvimi

## İndirme

Platformunuz için yükleyiciyi [GitHub Releases](https://github.com/conic-apps/launcher/releases)'den indirin:

| Platform | Mimari                | Formatlar                              |
| -------- | --------------------- | -------------------------------------- |
| Windows  | x64 · arm64           | MSI · NSIS yükleyici · taşınabilir exe |
| macOS    | Apple Silicon · Intel | DMG                                    |
| Linux    | x64 · arm64           | deb · rpm · AppImage                   |

Uygulama dahili otomatik güncelleme içerir — kurulumdan sonra manuel yükseltme gerekmez.

## Kaynak kodundan derleme

[Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/) ve [pnpm](https://pnpm.io/) kurulu olduğundan emin olun ve platformunuz için [Tauri ön koşullarını](https://tauri.app/start/prerequisites/) takip edin.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Geliştirme ve hata ayıklama
pnpm tauri build    # Üretim derlemesi
```

## Katkıda bulunma

[Issue'lar](https://github.com/conic-apps/launcher/issues/new/choose) ve Pull Request'ler (`dev` dalını hedefleyenler) hošgeldiniz. Göndermeden önce lütfen şunu çalıştırın:

```bash
pnpm check
```

## Mimari

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Mimari diyagramı: Vue 3 ön yüzü Tauri IPC üzerinden Rust çekirdeği ile iletişim kurar, altında alanlara ayrılmış crates modülleri">
</div>

Ön yüz (Vue 3 + Pinia), Tauri IPC üzerinden Rust yetenekleriyle iletişim kurar. `core`, Tauri uygulama montajını yönetirken, belirli yetenekler alanlara göre `crates/*` çalışma alanı modüllerine bölünmüştür. Bu modüller bağımsız olarak gelişir ve ihtiyaçlarına göre birleştirilir.

## Lisans

Bu proje [GPL-3.0](../../LICENSE) altında, GPLv3 Bölüm 7'deki ek koşullarla dağıtılır:

1. Değiştirilmiş sürümleri dağıtırken, orijinalden ayırt etmek için yazılım adını veya sürüm numarasını makul bir şekilde değiştirmeniz gerekir ([GPL-3.0 §7(c)](../../LICENSE)'e uygun olarak); kaynak kodundaki tüm proje adı referanslarını değiştirmeniz gerekir.
2. Yazılımda görüntülenen telif hakkı bildirimlerini kaldırmamalısınız ([GPL-3.0 §7(b)](../../LICENSE)'e uygun olarak).
3. Birisi alıcılarla sözleşme niteliğinde koşullara girip sorumluluk üstlenirse, lisans veren ve yazarlar müteselsil sorumluluk taşımazlar ([GPL-3.0 §7(b)](../../LICENSE)'e uygun olarak).

## Teşekkürler

- [Catppuccin](https://catppuccin.com) — Güzel pastel renk paleti temaları
- [BMCLAPI](https://bmclapi2.bangbang93.com/) ve bangbang93 tarafından [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) — İndirme hızlandırma ve Forge kurulum önyükleyici
- [MCIM](https://mod.mcimirror.top) — CurseForge çeviri özetleri ve ayna önbellekleme
- Öneri ve geri bildirim sağlayan tüm oyuncular

## Sorumluluk reddi

Conic Launcher resmi bir Minecraft ürünü değildir ve Mojang Studios tarafından onaylanmamış veya ilişkilendirilmemiştir. "Minecraft", Mojang AB'nin tescilli ticari markasıdır. Bu projedeki Minecraft markasının kullanımı, Mojang Studios'un <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Marka ve Varlık yönergeleri</a>ne tamamen uygundur.
