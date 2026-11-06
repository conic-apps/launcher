<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — dört Catppuccin temasıyla ana arayüzü gösteren modern, çapraz platformlu bir Minecraft başlatıcı">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>Küçük, hızlı ve çevik bir Minecraft başlatıcı</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md"><strong>Türkçe</strong></a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Bu nedir

Conic Launcher, Rust ile geliştirilmiş ve arayüzü [Slint](https://slint.dev) ile oluşturulmuş, küçük,
hızlı ve çevik bir masaüstü Minecraft başlatıcısıdır. Küçük kurulum boyutu, hızlı başlangıç, düşük
kaynak kullanımı — karbon emisyonlarını azaltmaya bile yardımcı olur.

> Proje şu anda erken Alpha aşamasında; özellikler ve arayüz hızla gelişiyor. Deneyin ve sorunları
> bildirin.

## Özellikler

### Çoklu instance yönetimi

Instance'ları mod loader'a göre gruplandırılmış olarak görüntüleyin; sıralama, arama ve favoriler
desteklenir; her instance kendi özel ayarlarına sahip olabilir.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Tüm ana loader'ları destekleme

Forge, NeoForge, Fabric ve Quilt için tam destek.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod ve resource pack pazar yeri

Yerleşik Modrinth ve CurseForge araması: proje ayrıntılarını ve bağımlılıkları başlatıcının içinde
görüntüleyin, bir modu veya resource pack'i tek tıkla instance'a kurun ve yerel olarak kurulu modları
doğrudan yönetin.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus çapraz LAN çoklu oyuncu

Ek araç gerekmez: bir grup oluşturun, davet kodunu arkadaşınıza gönderin ve aynı dünyaya birlikte
girin. Başlatıcı ağ ortamınızı (NAT türü) algılar ve iyileştirme önerileri sunar.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Çoklu giriş yöntemleri

Microsoft hesabı girişi (cihaz kodu akışı), çevrimdışı giriş ve herhangi bir Authlib-Injector /
Yggdrasil harici kimlik doğrulama sunucusu desteklenir.

<img width="840" height="533" alt="Ekran görüntüsü 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Rahatsızlık vermeyen başlatma deneyimi

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Kayıtlar ve oyun verileri

Kayıtlı dünyaların haritalarına göz atın, veri paketlerini ve resource pack'leri yönetin, oyun ekran
görüntülerini görüntüleyin — hepsi başlatıcının içinde.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Arayüz ve kişiselleştirme

Dört Catppuccin teması (Mocha · Macchiato · Frappé · Latte) ve yüksek kontrastlı varyantlar; özel
başlatıcı arka planı ve arka plan müziği desteklenir.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## İndirme

Platformunuza uygun kurulum dosyasını
[GitHub Releases](https://github.com/conic-apps/launcher/releases) veya
[resmî web sitesinden](https://conicmc.app) indirin:

| Platform | Mimari                | Biçimler              |
| -------- | --------------------- | --------------------- |
| Windows  | x64 · arm64           | MSI · taşınabilir exe |
| macOS    | Apple Silicon · Intel | DMG                   |
| Linux    | x64 · arm64           | deb · rpm · AppImage  |

> macOS 11.0 (Big Sur) veya üzeri gerekir.

> Windows'ta tek dosyalı sürümü kullanıyorsanız sistemde geçerli bir `VCRUNTIME140.dll` bulunduğundan
> emin olun.

## Kaynaktan derleme

[Rust 1.88+](https://rustup.rs/) kurulu olduğundan emin olun; Linux'ta ayrıca
[derleme bağımlılıkları](../../CONTRIBUTING.md#linux-build-dependencies) gerekir.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Geliştirme ve hata ayıklama
cargo build --release  # Üretim derlemesi
```

## Katkıda bulunma

Katkılarınızı bekliyoruz! Nasıl başlayacağınızı, geliştirme ortamınızı nasıl kuracağınızı ve nasıl
pull request göndereceğinizi öğrenmek için [CONTRIBUTING.md](../../CONTRIBUTING.md) dosyasını okuyun.

## Lisans

Bu proje, [GPL-3.0](../../LICENSE) altında ve GPLv3 Bölüm 7 kapsamındaki tüm ek şartlarla birlikte
dağıtılmaktadır.

## Teşekkürler

- [Catppuccin](https://catppuccin.com) — yumuşak ve güzel tema renkleri
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) —
  Forge kurulum önyüklemesi (bangbang93)
- [MCIM](https://mod.mcimirror.top) — CurseForge Çince çevirileri ve ayna önbelleği
- [HMCL](https://github.com/HMCL-dev/HMCL) — çökme işleme
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — yerel mod ayrıştırma akışı için referans

## Yasal uyarı

Conic Launcher resmî bir Minecraft ürünü değildir ve Mojang Studios tarafından onaylanmamış veya onunla
ilişkili değildir. “Minecraft”, Mojang AB'nin ticari markasıdır. Bu projede Minecraft markasının her
türlü kullanımı Mojang Studios'un
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Marka ve Varlık
Yönergeleri</a>ne uygundur.
