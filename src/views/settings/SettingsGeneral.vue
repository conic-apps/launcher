<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div ref="generalSettings">
    <SettingGroup>
      <SettingItem :title="t('settings.general.language')" icon="language">
        <BaseDropdownSelect
          :placeholder="t('settings.general.followSystem')"
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
        :title="t('settings.general.autoUpdate')"
        :description="t('settings.general.autoUpdateDesc')">
        <BaseSwitch v-model="config.auto_update"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.general.updateChannel')"
        :description="t('settings.general.updateChannelDesc')"
        :disabled="!config.auto_update">
        <BaseSelect
          :display-name="[t('settings.general.stable'), t('settings.general.beta')]"
          :options="['stable', 'beta']"
          v-model="config.update_channel"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="t('settings.general.openSetupWizard')"
        :description="t('settings.general.openSetupWizardDesc')"
        :navigable="true"
        @click="navigationStore.navigate('setup')">
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import BaseDropdownSelect from "@/components/BaseDropdownSelect.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import BaseSelect from "@/components/BaseSelect.vue";
import { useNavigationStore } from "@/store/navigation";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const config = useConfigStore();
const navigationStore = useNavigationStore();
</script>

<style scoped lang="less"></style>
