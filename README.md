<div align="center">
  <img src="./docs/screenshots/hero.png" width="100%" alt="Conic Launcher —— 现代、跨平台的 Minecraft 启动器，图中以四种 Catppuccin 主题展示主界面">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="./LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>一款小快灵的 Minecraft 启动器</strong><br>
  多实例管理 · 模组与资源包市场 · 跨局域网联机 · 多主题支持</p>
</div>

---

## 这是什么

Conic Launcher 是一个现代的桌面版 Minecraft 启动器：核心逻辑由 Rust 实现，界面基于 Tauri 2 + Vue 3，安装包小、启动快、资源占用低，甚至有助于减少碳排放。

从创建实例、安装加载器，到搜索模组、邀请好友联机，再到启动游戏与统计游戏时长——整个流程都可以在一个应用里完成。

> 项目目前处于早期 Alpha 阶段，功能与界面仍在快速迭代，欢迎试用并反馈问题。

## 功能亮点

### 多实例管理

按模组加载器分组浏览实例，支持排序、搜索与收藏；每个实例都有独立的设置与专属背景，游戏时长与上次运行时间一目了然。

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### 支持所有主流加载器

选择 Minecraft 版本与加载器版本，剩下的交给启动器：Vanilla、Forge、NeoForge、Fabric、Quilt 全部支持，安装过程实时展示进度。

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### 模组与资源包市场

内置 Modrinth 与 CurseForge 搜索：浏览项目详情与依赖、阅读中文摘要翻译，一键把模组或资源包装进实例，也可以直接管理已安装的本地模组。

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus 跨局域网联机

不需要额外工具：创建小组、把邀请码发给好友，即可一起进入同一个世界。启动器会检测网络环境（NAT 类型）并给出改善建议。

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 多种登录方式

支持微软正版登录（设备码流程）、离线登录，以及任意 Authlib-Injector / Yggdrasil 外置认证服务器。账户支持 3D 皮肤预览、披风展示与皮肤上传。

<img width="840" height="533" alt="截屏2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 省心的启动体验

自动扫描本机 Java，也可以自动下载合适的运行时；内存自动分配（含 32 位 Java 上限保护）。启动过程全程可视化：下载游戏文件、安装加载器、补全依赖库、等待进程启动。

高级玩家也没有被遗忘——JVM 垃圾回收器、JVM 参数、游戏参数、类路径、包装命令、启动前后执行命令均可自定义。

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### 存档与游戏资料

在启动器内直接浏览存档的世界地图、管理数据包与资源包、查看游戏截图。

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### 界面与个性化

四种 Catppuccin 主题（Mocha · Macchiato · Frappé · Latte），另有高对比度变体，可跟随系统深浅色自动切换；支持自定义启动器背景。内置音乐播放器，播放本地音乐并附带频谱可视化。

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### 更多

- 应用内自动更新，游戏版本更新提醒
- 多线程下载，可调连接数与限速，支持镜像服务器与系统代理
- 全局搜索（`/`）与命令输入（`;`）
- 游戏时长统计与活动日历

## 下载

前往 [GitHub Releases](https://github.com/conic-apps/launcher/releases) 下载对应平台的安装包：

| 平台    | 架构                  | 格式                           |
| ------- | --------------------- | ------------------------------ |
| Windows | x64 · arm64           | MSI · NSIS 安装器 · 便携版 exe |
| macOS   | Apple Silicon · Intel | DMG                            |
| Linux   | x64 · arm64           | deb · rpm · AppImage           |

应用内置自动更新，安装后无需手动升级。

## 从源码构建

确保已安装 [Rust 1.88+](https://rustup.rs/)、[Node.js 24](https://nodejs.org/) 与 [pnpm](https://pnpm.io/)，并参照 [Tauri 文档](https://tauri.app/start/prerequisites/) 准备各平台依赖。

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # 开发调试
pnpm tauri build    # 构建生产版本
```

## 参与贡献

欢迎提交 [Issue](https://github.com/conic-apps/launcher/issues/new/choose) 与 Pull Request（请指向 `dev` 分支）。提交前请运行检查：

```bash
pnpm check
```

## 架构

<div align="center">
  <img src="./docs/screenshots/architecture.png" width="100%" alt="架构图：Vue 3 前端经 Tauri IPC 与 Rust core 通信，下层为按领域划分的 crates 模块">
</div>

前端（Vue 3 + Pinia）通过 Tauri IPC 调用 Rust 能力；`core` 负责 Tauri 应用装配，具体能力按领域拆分在 `crates/*` 工作区中，各模块独立演进、按需组合。

## 许可证

本项目基于 [GPL-3.0](./LICENSE) 分发，并附带 GPLv3 第 7 条下的附加条款：

1. 分发本软件的修改版本时，必须以合理方式更改软件名称或版本号，以与原始版本区分（对应 [GPL-3.0 §7(c)](./LICENSE)）；你需要替换源码中所有与本项目名称相关的字样。
2. 不得移除软件内展示的版权声明（对应 [GPL-3.0 §7(b)](./LICENSE)）。
3. 若任何人与接受者签订具有合同性质的条款并作出责任承诺，由其自行承担责任，许可人与作者不承担连带责任（对应 [GPL-3.0 §7(b)](./LICENSE)）。

## 致谢

- [Catppuccin](https://catppuccin.com) — 柔和好看的主题配色
- [BMCLAPI](https://bmclapi2.bangbang93.com/) 与 [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) by bangbang93 — 安装加速与 Forge 安装引导
- [MCIM](https://mod.mcimirror.top) — CurseForge 中文摘要翻译与镜像缓存
- 所有为项目提出建议与反馈的玩家

## 免责声明

Conic Launcher 不是官方的 Minecraft 产品，也未获得 Mojang Studios 的批准或关联。“Minecraft”是 Mojang AB 的商标，本项目对 Minecraft 品牌的任何使用均符合 Mojang Studios 的<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">品牌与资产指南</a>。
