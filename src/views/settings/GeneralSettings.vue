<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup>
      <SettingItem
        :title="$t('settings.general.language')"
        :description="$t('settings.general.languageDesc')"
        icon="language">
        <BaseSelect
          :display-name="['English', '简体中文']"
          :options="['en_us', 'zh_cn']"
          v-model="config.language"
          :default="1"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="$t('settings.general.updateChannel')"
        :description="$t('settings.general.updateChannelDesc')"
        icon="branch">
        <BaseSelect
          :display-name="[
            $t('settings.general.weekly'),
            $t('settings.general.snapshot'),
            $t('settings.general.release'),
          ]"
          :options="['Weekly', 'Snapshot', 'Release']"
          v-model="config.update_channel"
          :default="2"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="$t('settings.general.autoUpdate')"
        :description="$t('settings.general.autoUpdateDesc')"
        icon="time">
        <BaseSwitch v-model="config.auto_update"></BaseSwitch>
      </SettingItem>
      <SettingItem :title="$t('settings.general.checkUpdate')" icon="circle-up" :navigable="true">
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import BaseSelect from "@/components/base/BaseSelect.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";

const config = useConfigStore();
</script>

<style lang="less"></style>
