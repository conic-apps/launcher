<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <setting-group>
      <setting-item
        :title="$t('settings.download.maxConnections')"
        :description="$t('settings.download.maxConnectionsDesc')"
        icon="link">
        <TextInputBox
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_connection"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.download.maxDownloadSpeed')"
        :description="$t('settings.download.maxDownloadSpeedDesc')"
        icon="gauge">
        <TextInputBox
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_download_speed"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.download.mirrorServer')"
        :clickAble="true"
        :description="$t('settings.download.mirrorServerDesc')"
        icon="server"
        :last="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
    <setting-group :title="$t('settings.download.proxy')">
      <setting-item :title="$t('settings.download.useSystemProxy')" icon="globe">
        <toggle-switch></toggle-switch>
      </setting-item>
    </setting-group>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import ToggleSwitch from "@/components/ToggleSwitch.vue";
import TextInputBox from "@/components/TextInputBox.vue";
import { useConfigStore } from "@/store/config";
const config = useConfigStore();
</script>

<style lang="less" scoped></style>
