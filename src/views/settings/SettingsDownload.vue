<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="t('settings.download.title')">
      <SettingItem
        :title="t('settings.download.maxConnections')"
        :description="t('settings.download.maxConnectionsDesc')"
        icon="link">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_connections"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.download.maxSpeed')"
        :description="t('settings.download.maxSpeedDesc')"
        icon="speedometer">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_download_speed"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.download.mirror')"
        :navigable="true"
        :description="t('settings.download.mirrorDesc')"
        icon="server"
        icon-fill="none"
        :last="true">
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="t('settings.download.proxy')">
      <SettingItem
        :title="'使用系统代理（重启应用后生效）'"
        icon="globe"
        :description="t('settings.download.proxyNote')">
        <BaseSwitch v-model="config.download.use_system_proxy"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseInput from "@/components/BaseInput.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const config = useConfigStore();
</script>

<style lang="less" scoped></style>
