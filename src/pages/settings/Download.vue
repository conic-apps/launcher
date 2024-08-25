<template>
  <keep-alive>
    <div>
      <setting-group>
        <setting-item
          :title="$t('settings.download.maxConnections')"
          :description="$t('settings.download.maxConnectionsDesc')"
          icon="link">
          <TextInputBox
            width="100px"
            :number-only="true"
            v-model.number="config.download.max_connection">
          </TextInputBox>
        </setting-item>
        <setting-item
          :title="$t('settings.download.maxDownloadSpeed')"
          :description="$t('settings.download.maxDownloadSpeedDesc')"
          icon="gauge">
          <TextInputBox
            width="100px"
            :number-only="true"
            v-model.number="config.download.max_download_speed">
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
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import { useConfigStore } from "@/config";
const config = useConfigStore();
</script>

<style lang="less" scoped></style>
