<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup>
      <SettingItem
        :title="$t('settings.download.maxConnections')"
        :description="$t('settings.download.maxConnectionsDesc')"
        icon="link">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_connections"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.download.maxDownloadSpeed')"
        :description="$t('settings.download.maxDownloadSpeedDesc')"
        icon="speedometer">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_download_speed"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.download.mirrorServer')"
        :navigable="true"
        :description="$t('settings.download.mirrorServerDesc')"
        icon="server"
        icon-fill="none"
        :last="true">
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="$t('settings.download.proxy')">
      <SettingItem :title="$t('settings.download.useSystemProxy')" icon="globe">
        <BaseSwitch v-model="config.download.use_system_proxy"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseInput from "@/components/base/BaseInput.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
const config = useConfigStore();
</script>

<style lang="less" scoped></style>
