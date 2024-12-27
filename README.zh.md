# Conic Launcher

[![status](https://img.shields.io/badge/status-stable-blue.svg)](https://github.com/tauri-apps/tauri/tree/dev)
[![Conic Launcher Auto Build](https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build%20app&logo=github)](https://github.com/conic-apps/launcher/actions/workflows/build.yml)
[![License](https://img.shields.io/github/license/conic-apps/launcher.svg)](https://www.gnu.org/licenses/quick-guide-GPL-3.0.html)
![Contributors](https://img.shields.io/github/contributors/conic-apps/launcher?color=2green)
[![website](https://img.shields.io/badge/website-launcher.btlcraft.top-purple.svg)](https://launcher.btlcraft.top)
[![https://good-labs.github.io/greater-good-affirmation/assets/images/badge.svg](https://good-labs.github.io/greater-good-affirmation/assets/images/badge.svg)](https://good-labs.github.io/greater-good-affirmation)

[English README](./README.md)

请到 [官网](https://amethyst.btlcraft.top) 下载启动器！

## 特性

-   可以下载 `Minecraft`, `Forge`, `Fabric`, `Optifine`, `Quilt`, 会自动选择速度最快的镜像服务器安装游戏
-   **跨平台**，使用rust编写的启动器，能够在 Windows, MacOS 和 GNU/Linux 上运行，~~甚至可以在手机上运行~~
-   **宇宙无敌的下载速度**，在 Linux 内核的系统上安装游戏甚至只需要15秒
-   **实例管理**，支持分组等功能，轻松管理多个实例
-   **模组下载**，你可以直接在启动器内下载 Curseforge, Modrinth, FTB 上的模组
-   **与 PCL2, HMCL, Bakaxl 联机**
-   **支持多种账号系统**，内置支持 Microsoft 和 Mojang Yggdrasil API。内置支持 LittleSkin，您也可以自行添加新的第三方验证服务！

## 优势

-   **节约存储空间**，在多个实例中启用的相同材质包只存储一次
-   **自动测试导致无法启动的模组**，可以自动测试哪些模组会在游戏启动时导致崩溃
-   **自定义启动器外观**，启动器的每一寸角落都可以自定义
-   **在启动器中修改存档的游戏规则**，不需要在游戏内输入指令
-   **自定义下载源**，可以使用自建镜像源而无需自己编译

## 开源协议

该程序在 GPL-3.0 开源协议下发布, 同时附有附加条款.

### 附加条款 (依据 GPL-3.0 开源协议第七条)

1. 当您分发该程序的修改版本时, 您必须以一种合理的方式修改该程序的名称或版本号, 以示其与原始版本不同. (依据 [GPL-3.0, 7(c)](./LICENSE#L372-L374))

您需要自行在源码中查找所有与本程序名称相关的词语并替换为您自己程序的名称

2. 您不得移除该程序所显示的版权声明. (依据 [GPL-3.0, 7(b)](./LICENSE#L368-L370))

3. 如果您和接收者签了合同性质的东西，并提供责任承诺，则授权人和作者不受此责任连带. (依据[GPL-3.0, 7(b)](./LICENSE#L382-L386))

## 手动构建

请先确保已完成[预先准备](https://v2.tauri.app/start/prerequisites)。

执行以下命令来拉取代码：

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
```

执行以下命令来安装依赖（在项目文件夹下）：

```bash
yarn install
```

接下来只需要运行以下命令即可完成构建：

```bash
yarn tauri build
```

如果你想为此项目作贡献，执行 `yarn tauri dev` 来调试应用程序，在[此处](https://tauri.app/zh-cn/v1/guides/)查看详细信息。
