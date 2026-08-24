<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <!-- TODO:  -->
    <!-- <info-box :click-able="true" @click="openUrl('https://github.com/sponsors/conic-apps')"> -->
    <!-- <p -->
    <!--   :style="{ -->
    <!--     marginBottom: '8px', -->
    <!--     backgroundImage: `linear-gradient(-90deg, var(--ctp-${config.appearance.palette.toLowerCase()}-mauve), var(--ctp-${config.appearance.palette.toLowerCase()}-pink))`, -->
    <!--     backgroundClip: 'text', -->
    <!--     color: '#00000000', -->
    <!--     fontSize: '18px', -->
    <!--     fontWeight: 'bold', -->
    <!--   }"> -->
    <!--   {{ "赞助 Conic Launcher" }} -->
    <!-- </p> -->
    <!-- {{ -->
    <!--   "Conic Launcher 是自由项目，旨在带来更好的游戏体验，并帮助降低碳排放。项目由 ConicMC 的开发者开发和维护，您的赞助或贡献不仅能帮助 Conic Launcher 获得更好的发展，更能为自由软件事业及环境保护贡献力量。" -->
    <!-- }} -->
    <!-- </info-box> -->
    <SettingGroup title="反馈">
      <SettingItem
        :title="'提交反馈'"
        :description="'打开一个 GitHub Issue，如果报告 Bug 记得上传启动器日志'"
        icon="flag"
        :navigable="true"
        @click="openUrl('https://github.com/conic-apps/launcher/issues/new/choose')">
      </SettingItem>
      <SettingItem
        :title="'查看源代码'"
        :description="'在 GitHub 中查看启动器的源代码'"
        icon="github"
        :navigable="true"
        @click="openUrl('https://github.com/conic-apps/launcher')">
      </SettingItem>
      <SettingItem
        :title="'查看启动器日志'"
        :description="'打开启动器日志文件夹，报告问题时请上传日志'"
        icon="document-text"
        :navigable="true"
        @click="openLogFolder">
      </SettingItem>
    </SettingGroup>
    <SettingGroup title="鸣谢">
      <SettingItem
        :title="'osu!'"
        :description="'Conic Launcher 使用的用户界面交互风格受 osu! 的启发'"
        :navigable="true"
        @click="openUrl('https://osu.ppy.sh')">
        <template #icon>
          <img src="@/assets/images/osu! logo.png" alt="osu!" style="width: calc(100% - 12px)" />
        </template>
      </SettingItem>
      <SettingItem
        :title="'Catppuccin'"
        :description="'Conic Launcher 的四种内置配色方案来自 Catppuccin'"
        :navigable="true"
        @click="openUrl('https://catppuccin.com')">
        <template #icon>
          <img
            src="@/assets/images/catppuccin-1544x1544_circle.png"
            alt="osu!"
            style="width: calc(100% - 12px)" />
        </template>
      </SettingItem>
      <SettingItem
        :title="'BMCLAPI'"
        :description="'Conic Launcher 镜像列表的默认配置包含 bangbang93 提供的 BMCLAPI 作为部分 Minecraft 资源的下载加速源'"
        :navigable="true"
        @click="openUrl('https://bmclapidoc.bangbang93.com')">
        <template #icon>
          <img
            src="@/assets/images/bangbang93-avatar.jpeg"
            alt="osu!"
            style="width: calc(100% - 12px); border-radius: 10000px" />
        </template>
      </SettingItem>
    </SettingGroup>
    <div class="version-info">
      <div class="logo">
        <LogoFlat />
      </div>
      <p class="application-name">Conic Launcher</p>
      <p class="application-version">
        {{ appVersion ?? "0.0.0" }}
      </p>
      <p class="copyright">Copyright 2022-2026 ConicMC developers. All rights reserved.</p>
      <p class="text">
        Conic Launcher 不是官方的 Minecraft 产品，也未获得 Mojang Studios
        的批准或关联。“Minecraft”是 Mojang AB 的商标，本项目对 Minecraft 品牌的任何使用均符合 Mojang
        Studios 的<a
          href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines"
          @click.prevent="openUrl('https://www.minecraft.net/en-us/terms#terms-brand_guidelines')"
          >品牌与资产指南</a
        >
        。
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import LogoFlat from "@/assets/logo-flat.svg";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getDataLocation } from "@conic/folder";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { onMounted, ref } from "vue";

async function openLogFolder() {
  const dataLocation = await getDataLocation();
  invoke("open_path", { path: dataLocation.logs });
}

const appVersion = ref(null as null | string);
onMounted(async () => {
  appVersion.value = await getVersion();
});
</script>

<style lang="less" scoped>
.version-info {
  display: flex;
  width: 100%;
  align-items: center;
  flex-direction: column;
  margin-top: 32px;
  .logo svg {
    width: 24px;
    margin-bottom: 8px;
    fill: var(--ctp-text);
  }
  .application-name {
    font-size: 12px;
    font-weight: 600;
  }
  .application-version {
    font-size: 9px;
    font-weight: normal;
    margin-top: 2px;
  }
  .copyright {
    font-size: 9px;
    font-weight: normal;
    margin-top: 6px;
  }
  .text {
    font-size: 9px;
    font-weight: normal;
    margin-top: 6px;
    text-align: center;
    line-height: 1.5;
    width: calc(100% - 96px);
  }
}
</style>
