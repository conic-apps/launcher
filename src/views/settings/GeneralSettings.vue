<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup>
      <SettingItem :title="$t('settings.general.language')" icon="language">
        <BaseDropdownSelect
          :display-name="[
            'English',
            '简体中文',
            '繁體中文',
            '日本語',
            '한국어',
            'Deutsch',
            'Français',
            'Español',
            'Português (Brasil)',
            'Русский',
            'Türkçe',
            'Polski',
          ]"
          :options="[
            'en_us',
            'zh_cn',
            'zh_tw',
            'ja_jp',
            'ko_kr',
            'de_de',
            'fr_fr',
            'es_es',
            'pt_br',
            'ru_ru',
            'tr_tr',
            'pl_pl',
          ]"
          v-model="config.language"
          :default="1"></BaseDropdownSelect>
      </SettingItem>
      <SettingItem
        :title="$t('settings.general.updateChannel')"
        :description="$t('settings.general.updateChannelDesc')"
        icon="branch">
        <BaseDropdownSelect
          :display-name="[
            $t('settings.general.release'),
            $t('settings.general.snapshot'),
            $t('settings.general.weekly'),
          ]"
          :options="['Release', 'Snapshot', 'Weekly']"
          v-model="config.update_channel"
          :default="2"></BaseDropdownSelect>
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
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import BaseDropdownSelect from "@/components/base/BaseDropdownSelect.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";

const config = useConfigStore();
</script>

<style lang="less"></style>
