<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — 現代的でクロスプラットフォームな Minecraft ランチャー、4つの Catppuccin テーマでメイン画面を表示">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>小さく、速く、軽い Minecraft ランチャー</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md"><strong>日本語</strong></a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## これは何

Conic Launcher は Rust で実装され、UI を
[Slint](https://slint.dev)
で構築した、小さく速く軽いデスクトップ版 Minecraft ランチャーです。インストールサイズが小さく、起動が速く、リソース消費も少なく、CO₂ 排出量の削減にも役立ちます。

> 本プロジェクトは現在アーリー Alpha 段階です。機能と UI は活発に更新されているため、ぜひお試しいただき、問題があればご報告ください。

## 機能

### マルチインスタンス管理

MOD ローダーごとにグループ化してインスタンスを一覧表示し、並べ替え・検索・お気に入りに対応。インスタンスごとに個別の設定ができます。

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### すべての主要ローダーをサポート

Forge、NeoForge、Fabric、Quilt を完全にサポートしています。

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### MOD・リソースパックマーケットプレイス

Modrinth と CurseForge の検索を内蔵。ランチャー内でプロジェクトの詳細と依存関係を確認し、ワンクリックで MOD やリソースパックをインスタンスに導入でき、インストール済みのローカル MOD も直接管理できます。

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus クロス LAN マルチプレイ

追加のツールは不要です。グループを作成し、招待コードを友だちに送るだけで、同じワールドに一緒に入れます。ランチャーはネットワーク環境（NAT タイプ）を検出し、改善の提案を行います。

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 複数のログイン方法

Microsoft アカウントログイン（デバイスコードフロー）、オフラインログイン、および任意の Authlib-Injector /
Yggdrasil 外部認証サーバーに対応しています。

<img width="840" height="533" alt="スクリーンショット 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 快適な起動体験

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### セーブデータとゲーム情報

セーブデータのワールドマップを閲覧し、データパックとリソースパックを管理し、ゲームのスクリーンショットを表示できます。すべてランチャー内で完結します。

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### UI とカスタマイズ

4 種類の Catppuccin テーマ（Mocha · Macchiato · Frappé · Latte）とハイコントラスト版を用意。ランチャーの背景と BGM をカスタマイズできます。

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## ダウンロード

[GitHub Releases](https://github.com/conic-apps/launcher/releases)
または[公式サイト](https://conicmc.app)から、お使いのプラットフォーム向けのインストーラーをダウンロードしてください：

| プラットフォーム | アーキテクチャ        | 形式                 |
| ---------------- | --------------------- | -------------------- |
| Windows          | x64 · arm64           | MSI · ポータブル exe |
| macOS            | Apple Silicon · Intel | DMG                  |
| Linux            | x64 · arm64           | deb · rpm · AppImage |

> macOS 11.0（Big Sur）以降が必要です。

> Windows で単一ファイル版を使用する場合は、システムに有効な `VCRUNTIME140.dll` が存在することを確認してください。

## ソースからビルド

[Rust 1.88+](https://rustup.rs/) がインストールされていることを確認してください。Linux では[ビルド依存関係](../../CONTRIBUTING.md#linux-build-dependencies)も必要です。

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # 開発・デバッグ
cargo build --release  # 本番ビルド
```

## コントリビュート

コントリビュートを歓迎します！始め方、開発環境のセットアップ、Pull Request の送り方は
[CONTRIBUTING.md](../../CONTRIBUTING.md) をご覧ください。

## ライセンス

本プロジェクトは [GPL-3.0](../../LICENSE) の下で配布され、GPLv3 第 7 条に基づくすべての追加条項が適用されます。

## 謝辞

- [Catppuccin](https://catppuccin.com) — やわらかく美しいテーマカラー
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) by
  bangbang93 — Forge インストールのブートストラップ
- [MCIM](https://mod.mcimirror.top) — CurseForge の中国語翻訳とミラーキャッシュ
- [HMCL](https://github.com/HMCL-dev/HMCL) — クラッシュ処理
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — ローカル MOD 解析フローの参考

## 免責事項

Conic Launcher は公式の Minecraft 製品ではなく、Mojang Studios の承認や提携を受けたものではありません。「Minecraft」は Mojang
AB の商標です。本プロジェクトにおける Minecraft ブランドの使用は、Mojang
Studios の<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">ブランドおよびアセットガイドライン</a>に準拠しています。
