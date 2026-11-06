<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — 现代、跨平台的 Minecraft 启动器，图中以四种 Catppuccin 主题展示主界面">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>一款小快灵的 Minecraft 启动器</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <strong>简体中文</strong> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## 这是什么

Conic Launcher 是一款小快灵的桌面版 Minecraft 启动器，由 Rust 实现，界面基于
[Slint](https://slint.dev)
构建。体积小、启动快、资源占用低，甚至有助于减少碳排放。

> 项目目前处于早期 Alpha 阶段，功能与界面仍在快速迭代，欢迎试用并反馈问题。

## 功能亮点

### 多实例管理

按模组加载器分组浏览实例，支持排序、搜索与收藏；可为每个实例进行特殊设置。

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### 支持所有主流加载器

完整支持 Forge、NeoForge、Fabric、Quilt。

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### 模组与资源包市场

内置 Modrinth 与 CurseForge 搜索，可在启动器内浏览项目详情与依赖、一键把模组或资源包装进实例、直接管理已安装的本地模组。

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus 跨局域网联机

不需要额外工具：创建小组、把邀请码发给好友，即可一起进入同一个世界。启动器会检测网络环境（NAT 类型）并给出改善建议。

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 多种登录方式

支持微软正版登录（设备码流程）、离线登录，以及任意 Authlib-Injector /
Yggdrasil 外置认证服务器。

<img width="840" height="533" alt="截屏2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 省心的启动体验

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### 存档与游戏资料

在启动器内直接浏览存档的世界地图、管理数据包与资源包、查看游戏截图。

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### 界面与个性化

四种 Catppuccin 主题（Mocha · Macchiato · Frappé ·
Latte），及高对比度变体；支持自定义启动器背景和背景音乐。

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## 下载

前往 [GitHub Releases](https://github.com/conic-apps/launcher/releases)
或[官方网站](https://conicmc.app)下载对应平台的安装包：

| 平台    | 架构                  | 格式                 |
| ------- | --------------------- | -------------------- |
| Windows | x64 · arm64           | MSI · 便携版 exe     |
| macOS   | Apple Silicon · Intel | DMG                  |
| Linux   | x64 · arm64           | deb · rpm · AppImage |

> 需要 macOS 11.0（Big Sur）或更高版本。

> 若在 Windows 使用单文件版本，请确保系统中存在有效的 `VCRUNTIME140.dll`

## 从源码构建

确保已安装
[Rust 1.88+](https://rustup.rs/)；在 Linux 上，还需安装[构建依赖](../../CONTRIBUTING.md#linux-build-dependencies)。

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # 开发调试
cargo build --release  # 构建生产版本
```

## 参与贡献

欢迎参与贡献！请阅读 [CONTRIBUTING.md](../../CONTRIBUTING.md)
了解如何开始、配置开发环境以及提交 Pull Request。

## 许可证

本项目基于 [GPL-3.0](../../LICENSE) 分发，并附带 GPLv3 第 7 条下的所有附加条款。

## 致谢

- [Catppuccin](https://catppuccin.com) — 柔和好看的主题配色
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) by
  bangbang93 — Forge 安装引导
- [MCIM](https://mod.mcimirror.top) — CurseForge 中文翻译与镜像缓存
- [HMCL](https://github.com/HMCL-dev/HMCL) — 崩溃处理
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — 本地 Mod 解析流程

## 免责声明

Conic Launcher 不是官方的 Minecraft 产品，也未获得 Mojang Studios 的批准或关联。“Minecraft”是 Mojang
AB 的商标，本项目对 Minecraft 品牌的任何使用均符合 Mojang
Studios 的<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">品牌与资产指南</a>。
