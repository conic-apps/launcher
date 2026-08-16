<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup title="下载选项">
      <SettingItem
        :title="'最大连接数'"
        :description="'限制同时与服务器建立的连接数量，只对安装时的下载有效，<strong>过大反而会降低速度！</strong>'"
        icon="link">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_connections"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="'最大下载速度(单位：B/s)'"
        :description="'限制下载速度，只对安装时的下载有效，必须大于1024才有效，设置为 0 以禁用'"
        icon="speedometer">
        <BaseInput
          width="100px"
          :number-only="true"
          v-model.number="config.download.max_download_speed"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="'镜像服务器'"
        :navigable="true"
        :description="'从其他地方下载游戏文件'"
        icon="server"
        icon-fill="none"
        :last="true">
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="'代理'">
      <SettingItem
        :title="'使用系统代理（重启应用后生效）'"
        icon="globe"
        description="目前仅支持通过环境变量设置代理">
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
const config = useConfigStore();
</script>

<style lang="less" scoped></style>
