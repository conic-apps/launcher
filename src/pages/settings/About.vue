<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <info-box :click-able="true" @click="openUrl('https://afdian.com/a/BrokenDeer')">
      <h2
        :style="{
          'margin-bottom': '8px',
          'background-image': `linear-gradient(-90deg, var(--ctp-${config.appearance.palette.toLowerCase()}-mauve), var(--ctp-${config.appearance.palette.toLowerCase()}-pink))`,
          'background-clip': 'text',
          color: '#00000000',
        }">
        {{ $t("settings.about.sponsorTitle") }}
      </h2>
      {{ $t("settings.about.sponsorDesc") }}
    </info-box>
    <setting-group>
      <setting-item
        :title="$t('settings.about.report')"
        :description="$t('settings.about.reportDesc')"
        icon="flag"
        :clickAble="true"
        @click="openUrl('https://github.com/conic-apps/launcher')">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </setting-item>
      <setting-item
        :title="$t('settings.about.viewSourceCode')"
        :description="$t('settings.about.viewSourceCodeDesc')"
        icon="github"
        :clickAble="true"
        @click="openUrl('https://github.com/conic-apps/launcher/issues/new/choose')">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </setting-item>
    </setting-group>
    <SettingGroup :title="$t('settings.about.thirdPartyLibraries')">
      <setting-item
        title="Tauri"
        description="Copyright 2022 Tauri Programme within The Commons Conservancy<br> Licensed under the Apache-2.0 License or MIT Licenses"></setting-item>
      <setting-item
        title="Vue"
        description="Copyright (c) 2018-present, Yuxi (Evan) You and Vue contributors<br> Licensed under the MIT License"></setting-item>
      <setting-item
        title="vite"
        description="Copyright (c) 2019-present, Yuxi (Evan) You and Vite contributors<br> Licensed under the MIT License"></setting-item>
      <setting-item
        title="Less"
        description="Copyright (c) 2009-2017 Alexis Sellier & The Core Less Team<br> Licensed under the Apache License."></setting-item>
      <setting-item
        title="Tokio"
        description="Copyright (c) 2019 The Tokio Project Developers<br> Licensed under the MIT License."></setting-item>
      <setting-item
        title="Serde"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="Anyhow"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="OneCell"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="Rejex"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="Rayon"
        description="Rayon is distributed under the terms of both the MIT license and the Apache License (Version 2.0)."></setting-item>
      <setting-item
        title="env_logger"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import InfoBox from "@/components/InfoBox.vue";
import AppIcon from "@/components/AppIcon.vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useConfigStore } from "@/store/config";

const config = useConfigStore();
</script>
