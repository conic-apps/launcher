<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher —— 現代、跨平台的 Minecraft 啟動器，圖中以四種 Catppuccin 主題展示主介面">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>一款小快靈的 Minecraft 啟動器</strong><br>
  多實例管理 · 模組與資源包市場 · 跨區域網路連線 · 多主題支援</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md"><strong>繁體中文</strong></a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## 這是什麼

Conic Launcher 是一個現代的桌面版 Minecraft 啟動器：核心邏輯由 Rust 實現，介面基於 Tauri 2 + Vue 3，安裝包小、啟動快、資源佔用低，甚至有助於減少碳排放。

從建立實例、安裝載入器，到搜尋模組、邀請好友連線，再到啟動遊戲與統計遊戲時長——整個流程都可以在一個應用裡完成。

> 專案目前處於早期 Alpha 階段，功能與介面仍在快速迭代，歡迎試用並回饋問題。

## 功能亮點

### 多實例管理

按模組載入器分組瀏覽實例，支援排序、搜尋與收藏；每個實例都有獨立的設定與專屬背景，遊戲時長與上次執行時間一目了然。

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### 支援所有主流載入器

選擇 Minecraft 版本與載入器版本，剩下的交給啟動器：Vanilla、Forge、NeoForge、Fabric、Quilt 全部支援，安裝過程即時展示進度。

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### 模組與資源包市場

內建 Modrinth 與 CurseForge 搜尋：瀏覽專案詳情與依賴、閱讀翻譯摘要，一鍵把模組或資源包裝進實例，也可以直接管理已安裝的本地模組。

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus 跨區域網路連線

不需要額外工具：建立小組、把邀請碼發給好友，即可一起進入同一個世界。啟動器會偵測網路環境（NAT 類型）並給出改善建議。

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 多種登入方式

支援微軟正版登入（裝置碼流程）、離線登入，以及任意 Authlib-Injector / Yggdrasil 外部認證伺服器。帳戶支援 3D 鬥篷展示與皮膚上傳。

<img width="840" height="533" alt="截圖2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 省心的啟動體驗

自動掃描本機 Java，也可以自動下載合適的運行時；記憶體自動分配（含 32 位元 Java 上限保護）。啟動過程全程可視化：下載遊戲檔案、安裝載入器、補全依賴庫、等待行程啟動。

進階玩家也沒有被遺忘——JVM 垃圾回收器、JVM 參數、遊戲參數、類別路徑、包裝命令、啟動前後執行命令均可自訂。

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### 存檔與遊戲資料

在啟動器內直接瀏覽存檔的世界地圖、管理資料包與資源包、檢視遊戲截圖。

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### 介面與個人化

四種 Catppuccin 主題（Mocha · Macchiato · Frappé · Latte），另有高對比度變體，可跟隨系統深淺色自動切換；支援自訂啟動器背景。內建音樂播放器，播放本地音樂並附帶頻譜視覺化。

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### 更多

- 應用內自動更新，遊戲版本更新提醒
- 多執行緒下載，可調連線數與限速，支援鏡像伺服器與系統代理
- 全域搜尋（`/`）與命令輸入（`;`）
- 遊戲時長統計與活動日曆

## 下載

前往 [GitHub Releases](https://github.com/conic-apps/launcher/releases) 下載對應平台的安裝包：

| 平台    | 架構                  | 格式                           |
| ------- | --------------------- | ------------------------------ |
| Windows | x64 · arm64           | MSI · NSIS 安裝器 · 便攜版 exe |
| macOS   | Apple Silicon · Intel | DMG                            |
| Linux   | x64 · arm64           | deb · rpm · AppImage           |

應用內建自動更新，安裝後無需手動升級。

## 從原始碼建構

確保已安裝 [Rust 1.88+](https://rustup.rs/)、[Node.js 24](https://nodejs.org/) 與 [pnpm](https://pnpm.io/)，並參照 [Tauri 文件](https://tauri.app/start/prerequisites/) 準備各平台依賴。

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # 開發除錯
pnpm tauri build    # 建構正式版本
```

## 參與貢獻

歡迎提交 [Issue](https://github.com/conic-apps/launcher/issues/new/choose) 與 Pull Request（請指向 `dev` 分支）。提交前請執行檢查：

```bash
pnpm check
```

## 架構

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="架構圖：Vue 3 前端經 Tauri IPC 與 Rust core 通訊，下層為按領域劃分的 crates 模組">
</div>

前端（Vue 3 + Pinia）透過 Tauri IPC 呼叫 Rust 能力；`core` 負責 Tauri 應用組裝，具體能力按領域拆分在 `crates/*` 工作區中，各模組獨立演進、按需組合。

## 授權條款

本專案基於 [GPL-3.0](../../LICENSE) 分發，並附帶 GPLv3 第 7 條下的附加條款：

1. 分發本軟體的修改版本時，必須以合理方式更改軟體名稱或版本號，以與原始版本區分（對應 [GPL-3.0 §7(c)](../../LICENSE)）；你需要替換原始碼中所有與本專案名稱相關的字樣。
2. 不得移除軟體內展示的版權聲明（對應 [GPL-3.0 §7(b)](../../LICENSE)）。
3. 若任何人與接受者簽訂具有合約性質的條款並作出責任承諾，由其自行承擔責任，許可人與作者不承擔連帶責任（對應 [GPL-3.0 §7(b)](../../LICENSE)）。

## 致謝

- [Catppuccin](https://catppuccin.com) — 柔和好看的主題配色
- [BMCLAPI](https://bmclapi2.bangbang93.com/) 與 bangbang93 的 [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) — 安裝加速與 Forge 安裝引導
- [MCIM](https://mod.mcimirror.top) — CurseForge 翻譯摘要與鏡像快取
- 所有為專案提出建議與回饋的玩家

## 免責聲明

Conic Launcher 不是官方的 Minecraft 產品，也未獲得 Mojang Studios 的批准或關聯。「Minecraft」是 Mojang AB 的商標，本專案對 Minecraft 品牌的任何使用均符合 Mojang Studios 的<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">品牌與資產指南</a>。
