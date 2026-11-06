<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — 現代、跨平台的 Minecraft 啟動器，圖中以四種 Catppuccin 主題展示主介面">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>一款小快靈的 Minecraft 啟動器</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md"><strong>繁體中文</strong></a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## 這是什麼

Conic Launcher 是一款小快靈的桌面版 Minecraft 啟動器，由 Rust 實現，介面基於
[Slint](https://slint.dev)
建構。體積小、啟動快、資源佔用低，甚至有助於減少碳排放。

> 專案目前處於早期 Alpha 階段，功能與介面仍在快速迭代，歡迎試用並回饋問題。

## 功能亮點

### 多實例管理

按模組載入器分組瀏覽實例，支援排序、搜尋與收藏；可為每個實例進行特殊設定。

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### 支援所有主流載入器

完整支援 Forge、NeoForge、Fabric、Quilt。

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### 模組與資源包市場

內建 Modrinth 與 CurseForge 搜尋，可在啟動器內瀏覽專案詳情與依賴、一鍵把模組或資源包裝進實例、直接管理已安裝的本地模組。

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus 跨區域網路連線

不需要額外工具：建立小組、把邀請碼發給好友，即可一起進入同一個世界。啟動器會偵測網路環境（NAT 類型）並給出改善建議。

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 多種登入方式

支援微軟正版登入（裝置碼流程）、離線登入，以及任意 Authlib-Injector /
Yggdrasil 外部認證伺服器。

<img width="840" height="533" alt="截圖 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 省心的啟動體驗

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### 存檔與遊戲資料

在啟動器內直接瀏覽存檔的世界地圖、管理資料包與資源包、檢視遊戲截圖。

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### 介面與個人化

四種 Catppuccin 主題（Mocha · Macchiato · Frappé ·
Latte），及高對比度變體；支援自訂啟動器背景和背景音樂。

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## 下載

前往 [GitHub Releases](https://github.com/conic-apps/launcher/releases)
或[官方網站](https://conicmc.app)下載對應平台的安裝包：

| 平台    | 架構                  | 格式                 |
| ------- | --------------------- | -------------------- |
| Windows | x64 · arm64           | MSI · 便攜版 exe     |
| macOS   | Apple Silicon · Intel | DMG                  |
| Linux   | x64 · arm64           | deb · rpm · AppImage |

> 需要 macOS 11.0（Big Sur）或以上版本。

> 若在 Windows 使用單一檔案版本，請確保系統中存在有效的 `VCRUNTIME140.dll`

## 從原始碼建構

請確保已安裝
[Rust 1.88+](https://rustup.rs/)；在 Linux 上，還需安裝[建構相依套件](../../CONTRIBUTING.md#linux-build-dependencies)。

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # 開發除錯
cargo build --release  # 建構正式版本
```

## 參與貢獻

歡迎參與貢獻！請閱讀 [CONTRIBUTING.md](../../CONTRIBUTING.md)
了解如何開始、設定開發環境以及提交 Pull Request。

## 授權條款

本專案基於 [GPL-3.0](../../LICENSE) 分發，並附帶 GPLv3 第 7 條下的所有附加條款。

## 致謝

- [Catppuccin](https://catppuccin.com) — 柔和好看的主題配色
- bangbang93 的
  [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) —
  Forge 安裝引導
- [MCIM](https://mod.mcimirror.top) — CurseForge 中文翻譯與鏡像快取
- [HMCL](https://github.com/HMCL-dev/HMCL) — 崩潰處理
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — 本地 Mod 解析流程

## 免責聲明

Conic Launcher 不是官方的 Minecraft 產品，也未獲得 Mojang
Studios 的批准或關聯。「Minecraft」是 Mojang
AB 的商標，本專案對 Minecraft 品牌的任何使用均符合 Mojang
Studios 的<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">品牌與資產指南</a>。
