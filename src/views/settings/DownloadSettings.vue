<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup title="下载选项">
      <SettingItem
        :title="$t('settings.network.maxConnections')"
        :description="$t('settings.network.maxConnectionsDesc')"
        icon="link">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_connections"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.network.maxDownloadSpeed')"
        :description="$t('settings.network.maxDownloadSpeedDesc')"
        icon="speedometer">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_download_speed"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.network.mirrorServer')"
        :navigable="true"
        :description="$t('settings.network.mirrorServerDesc')"
        icon="server"
        icon-fill="none"
        :last="true">
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="$t('settings.network.proxy')">
      <SettingItem :title="$t('settings.network.useSystemProxy')" icon="globe">
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
