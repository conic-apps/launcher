<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <info-box :click-able="true" @click="openUrl('https://afdian.com/a/BrokenDeer')">
      <p
        :style="{
          marginBottom: '8px',
          backgroundImage: `linear-gradient(-90deg, var(--ctp-${config.appearance.palette.toLowerCase()}-mauve), var(--ctp-${config.appearance.palette.toLowerCase()}-pink))`,
          backgroundClip: 'text',
          color: '#00000000',
          fontSize: '18px',
          fontWeight: 'bold',
        }">
        {{ $t("settings.about.sponsorTitle") }}
      </p>
      {{ $t("settings.about.sponsorDesc") }}
    </info-box>
    <SettingGroup>
      <SettingItem
        :title="$t('settings.about.report')"
        :description="$t('settings.about.reportDesc')"
        icon="flag"
        :navigable="true"
        @click="openUrl('https://github.com/conic-apps/launcher')">
      </SettingItem>
      <SettingItem
        :title="$t('settings.about.viewSourceCode')"
        :description="$t('settings.about.viewSourceCodeDesc')"
        icon="github"
        :navigable="true"
        @click="openUrl('https://github.com/conic-apps/launcher/issues/new/choose')">
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="$t('settings.about.thirdPartyLibraries')">
      <SettingItem
        title="Tauri"
        description="Copyright 2022 Tauri Programme within The Commons Conservancy<br> Licensed under the Apache-2.0 License or MIT Licenses"></SettingItem>
      <SettingItem
        title="Vue"
        description="Copyright (c) 2018-present, Yuxi (Evan) You and Vue contributors<br> Licensed under the MIT License"></SettingItem>
      <SettingItem
        title="vite"
        description="Copyright (c) 2019-present, Yuxi (Evan) You and Vite contributors<br> Licensed under the MIT License"></SettingItem>
      <SettingItem
        title="Less"
        description="Copyright (c) 2009-2017 Alexis Sellier & The Core Less Team<br> Licensed under the Apache License."></SettingItem>
      <SettingItem
        title="Tokio"
        description="Copyright (c) 2019 The Tokio Project Developers<br> Licensed under the MIT License."></SettingItem>
      <SettingItem
        title="Serde"
        description="Licensed under Apache-2.0 or MIT license."></SettingItem>
      <SettingItem
        title="Anyhow"
        description="Licensed under Apache-2.0 or MIT license."></SettingItem>
      <SettingItem
        title="OneCell"
        description="Licensed under Apache-2.0 or MIT license."></SettingItem>
      <SettingItem
        title="Rejex"
        description="Licensed under Apache-2.0 or MIT license."></SettingItem>
      <SettingItem
        title="Rayon"
        description="Rayon is distributed under the terms of both the MIT license and the Apache License (Version 2.0)."></SettingItem>
      <SettingItem
        title="env_logger"
        description="Licensed under Apache-2.0 or MIT license."></SettingItem>
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
