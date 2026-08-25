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
    <SettingGroup :title="t('settings.about.feedback')">
      <SettingItem
        :title="t('settings.about.submitFeedback')"
        :description="t('settings.about.submitFeedbackDesc')"
        icon="flag"
        :navigable="true"
        @click="openUrl('https://github.com/conic-apps/launcher/issues/new/choose')">
      </SettingItem>
      <SettingItem
        :title="t('settings.about.viewSource')"
        :description="t('settings.about.viewSourceDesc')"
        icon="github"
        :navigable="true"
        @click="openUrl('https://github.com/conic-apps/launcher')">
      </SettingItem>
      <SettingItem
        :title="t('settings.about.viewLogs')"
        :description="t('settings.about.viewLogsDesc')"
        icon="document-text"
        :navigable="true"
        @click="openLogFolder">
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="t('settings.about.credits')">
      <SettingItem
        :title="'osu!'"
        :description="t('settings.about.creditsOsu')"
        :navigable="true"
        @click="openUrl('https://osu.ppy.sh')">
        <template #icon>
          <img src="@/assets/images/osu! logo.png" alt="osu!" style="width: calc(100% - 12px)" />
        </template>
      </SettingItem>
      <SettingItem
        :title="'Catppuccin'"
        :description="t('settings.about.creditsCatppuccin')"
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
        :description="t('settings.about.creditsBMCLAPI')"
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
        {{ t("settings.about.disclaimer") }}
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
import { useI18n } from "vue-i18n";

const { t } = useI18n();

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
