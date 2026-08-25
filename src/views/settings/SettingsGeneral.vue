<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div ref="generalSettings">
    <SettingGroup>
      <SettingItem :title="'语言'" icon="language">
        <BaseDropdownSelect
          placeholder="跟随系统设置"
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
      <SettingItem :title="'自动更新'" :description="'有更新可用时自动下载并安装'">
        <BaseSwitch v-model="config.auto_update"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="'更新通道'"
        :description="'控制启动器获取更新的频率'"
        :disabled="!config.auto_update">
        <BaseSelect
          :display-name="['正式版', '测试版']"
          :options="['stable', 'beta']"
          v-model="config.update_channel"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="'打开设置向导'"
        :description="'设置向导可带你完成基本的启动器设置'"
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

const config = useConfigStore();
const navigationStore = useNavigationStore();
</script>

<style scoped lang="less"></style>
