<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher —— 現代的でクロスプラットフォームな Minecraft ランチャー、4つの Catppuccin テーマでメイン画面を表示">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>小さく、速く、軽い Minecraft ランチャー</strong><br>
  マルチインスタンス管理 · Mod・リソースパックマーケットプレイス · クロスLANマルチプレイ · マルチテーマ対応</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md"><strong>日本語</strong></a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## これは何

Conic Launcher はモダンなデスクトップ Minecraft ランチャーです。コアロジックは Rust で実装され、UI は Tauri 2 + Vue 3 で構成されています。インストールサイズが小さく、起動が高速で、リソース消費が少なく、 carbon emission の削減にも貢献します。

インスタンスの作成、ローダーのインストール、Mod の検索、フレンドを招待して一緒にプレイ、ゲームの起動とプレイ時間の統計まで——すべてのワークフローを一つのアプリで完結できます。

> プロジェクトは現在早期アルファ段階です。機能と UI は急速に迭代されていますので、ぜひ試してフィードバックをお寄せください。

## 機能

### マルチインスタンス管理

Mod ローダーごとにグループ分けされたインスタンスを閲覧でき、ソート、検索、お気に入りに対応。各インスタンスには個別の設定、カスタム背景、プレイ時間、最終プレイ日時が一目でわかります。

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### すべての主要ローダーをサポート

Minecraft バージョンとローダーバージョンを選択するだけで、残りはランチャーが処理します。Vanilla、Forge、NeoForge、Fabric、Quilt すべて対応し、インストール進捗をリアルタイムで表示します。

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod・リソースパックマーケットプレイス

Modrinth と CurseForge の組み込み検索：プロジェクトの詳細や依存関係を閲覧、翻訳された概要を読み、ワンクリックで Mod やリソースパックをインスタンスにインストール。ローカルにインストールされた Mod の管理も直接できます。

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus クロスLANマルチプレイ

追加ツールは不要：グループを作成し、招待コードをフレンドに送るだけで、同じワールドで一緒にプレイできます。ランチャーはネットワーク環境（NAT タイプ）を検出し、改善提案を行います。

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 複数のログイン方法

Microsoft アカウントログイン（デバイスコードフロー）、オフラインログイン、Authlib-Injector / Yggdrasil 外部認証サーバーに対応。アカウントには3D スキンプレビューやケープ表示、スキンアップロード機能があります。

<img width="840" height="533" alt="スクリーンショット 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 快適な起動体験

システム上の Java を自動スキャンし、適切なランタイムを自動ダウンロードすることもできます。メモリは自動割り当て（32ビット Java の上限保護付き）。起動プロセスは完全に可視化：ゲームファイルのダウンロード、ローダーのインストール、依存関係の解決、プロセスの起動待ち。

上級ユーザーも見逃されません——JVM ガベージコレクタ、JVM 引数、ゲーム引数、クラスパス、ラッパー コマンド、起動前后のコマンド実行がすべてカスタマイズ可能です。

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### セーブデータとゲーム情報

ランチャー内でセーブワールドのマップ閲覧、データパックやリソースパックの管理、ゲームスクリーンショットの表示が可能です。

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### UIとカスタマイズ

4つの Catppuccin テーマ（Mocha · Macchiato · Frappé · Latte）、ハイコントラストバリアント、システム設定に連動したライト/ダークモード自動切り替え。カスタムランチャー背景にも対応。ローカルミュージック再生とスペクトラム可視化機能を備えた音楽プレイヤー内蔵。

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### その他

- アプリ内自動更新、ゲームバージョン更新通知
- マルチスレッドダウンロード、接続数と速度制限の調整、ミラーサーバーとシステムプロキシ対応
- グローバル検索（`/`）とコマンドパレット（`;`）
- プレイ時間統計とアクティビティカレンダー

## ダウンロード

[GitHub Releases](https://github.com/conic-apps/launcher/releases) からお使いのプラットフォームに対応するインストーラーをダウンロード：

| プラットフォーム | アーキテクチャ        | フォーマット                     |
| --------------- | --------------------- | -------------------------------- |
| Windows         | x64 · arm64           | MSI · NSIS インストーラー · ポータブル exe |
| macOS           | Apple Silicon · Intel | DMG                              |
| Linux           | x64 · arm64           | deb · rpm · AppImage             |

アプリには自動更新機能が組み込まれており、インストール後の手動アップグレードは不要です。

## ソースからビルド

[Rust 1.88+](https://rustup.rs/)、[Node.js 24](https://nodejs.org/)、[pnpm](https://pnpm.io/) がインストールされていることを確認し、[Tauri の前提条件](https://tauri.app/start/prerequisites/) に従って各プラットフォームの依存関係を準備してください。

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # 開発・デバッグ
pnpm tauri build    # 本番ビルド
```

## コントリビュート

[Issue](https://github.com/conic-apps/launcher/issues/new/choose) や Pull Request（`dev` ブランチを対象）の提出を歓迎します。提出前に以下を実行してください：

```bash
pnpm check
```

## アーキテクチャ

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="アーキテクチャ図：Vue 3 フロントエンドが Tauri IPC 経由で Rust コアと通信、下位にドメイン別 crates モジュール">
</div>

フロントエンド（Vue 3 + Pinia）は Tauri IPC 経由で Rust の機能と通信します。`core` は Tauri アプリの組み立てを担当し、具体的な機能はドメインごとに `crates/*` ワークスペースモジュールに分割され、独立して進化し、必要に応じて組み合わせられます。

## ライセンス

本プロジェクトは [GPL-3.0](../../LICENSE) で配布され、GPLv3 第7条の追加条項が適用されます：

1. 変更版を配布する場合、元のバージョンと区別できるよう、合理的にソフトウェア名またはバージョン番号を変更する必要があります（[GPL-3.0 §7(c)](../../LICENSE) に対応）。ソースコード内のプロジェクト名に関連するすべての参照を置換する必要があります。
2. ソフトウェアに表示されている著作権表示を削除してはなりません（[GPL-3.0 §7(b)](../../LICENSE) に対応）。
3. 誰かが受容者と契約上の条件を締結して責任を負った場合、ライセンサーと著者は連帯責任を負いません（[GPL-3.0 §7(b)](../../LICENSE) に対応）。

## 謝辞

- [Catppuccin](https://catppuccin.com) — 美しいパステルカラーテーマ
- [BMCLAPI](https://bmclapi2.bangbang93.com/) と bangbang93 による [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) — ダウンロード加速と Forge インストールブートストラップ
- [MCIM](https://mod.mcimirror.top) — CurseForge 翻訳概要とミラーキャッシング
- 提案やフィードバックをしていただいたすべてのプレイヤーの皆さん

## 免責事項

Conic Launcher は Minecraft の公式製品ではなく、Mojang Studios によって承認または関連付けられたものではありません。「Minecraft」は Mojang AB の商標であり、本プロジェクトにおける Minecraft ブランドの使用はすべて Mojang Studios の<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">ブランドとアセットガイドライン</a>に準拠しています。
