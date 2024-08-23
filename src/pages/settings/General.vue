<template>
  <keep-alive>
    <div>
      <setting-group>
        <setting-item :title="$t('settings.general.language')" :description="$t('settings.general.languageDesc')" icon="language">
          <select-vue :display-name="['English', '简体中文', 'Русский']" :options="['en', 'zh_cn', 'ru_ru']"
            v-model="config.language" :default="1"></select-vue>
        </setting-item>
        <setting-item :title="$t('settings.general.updateChannel')"
          :description="$t('settings.general.updateChannelDesc')" icon="radio">
          <select-vue :display-name="[
            $t('settings.general.weekly'),
            $t('settings.general.snapshot'),
            $t('settings.general.release'),
          ]" :options="['Weekly', 'Snapshot', 'Release']" v-model="config.update_channel" :default="2"></select-vue>
        </setting-item>
        <setting-item :title="$t('settings.general.autoUpdate')" :description="$t('settings.general.autoUpdateDesc')"
          icon="clock">
          <toggle-switch v-model="config.auto_update"></toggle-switch>
        </setting-item>
        <setting-item :title="$t('settings.general.checkUpdate')" icon="circle-up" :clickAble="true">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SelectVue from "@/components/controllers/Select.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/config";

const config = useConfigStore();
</script>

<style lang="less"></style>
