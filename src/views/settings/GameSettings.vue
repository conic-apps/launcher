<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="$t('settings.game.jvmTitle')">
      <SettingItem
        :title="$t('settings.game.chooseJava')"
        icon="java"
        :icon-size="36"
        :navigable="true">
        <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{
          $t("settings.game.selectedJava", {
            selected: "Java 17",
          })
        }}</span>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.addJava')"
        :description="$t('settings.game.addJavaDesc')"
        icon="cloud-download"
        :navigable="true">
      </SettingItem>
      <!-- <SettingItem title="Java 内存"></SettingItem> -->
    </SettingGroup>
    <SettingGroup :title="$t('settings.game.launchOptions')">
      <SettingItem
        :title="$t('settings.game.launcherName')"
        :description="$t('settings.game.launcherNameDesc')"
        icon="tag">
        <BaseInput
          width="260px"
          v-model="config.launch.launcher_name"
          :lazy-update-value="true"></BaseInput>
      </SettingItem>
      <!-- TODO:<SettingItem title="服务器地址" description="启动后自动加入服务器" icon="server"> -->
      <!--   <BaseInput -->
      <!--     width="240px" -->
      <!--     v-model="config.launch.server!.ip" -->
      <!--     style="display: inline-block; margin-right: 16px" -->
      <!--     placeholder="IP 或域名"></BaseInput> -->
      <!--   <BaseInput -->
      <!--     width="100px" -->
      <!--     v-model="config.launch.server!.port" -->
      <!--     placeholder="端口" -->
      <!--     style="display: inline-block"></BaseInput> -->
      <!-- </SettingItem> -->
      <SettingItem
        :title="$t('settings.game.enterWorldAfterLaunch')"
        :description="$t('settings.game.enterWorldAfterLaunchDesc')"
        icon="enter">
        <BaseInput
          width="260px"
          :placeholder="$t('settings.game.enterWorldAfterLaunchPlaceholder')"
          :lazy-update-value="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.fullscreen')"
        :description="$t('settings.game.fullscreenDesc')"
        icon="expand">
        <BaseSwitch v-model="config.launch.fullscreen"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :disabled="config.launch.fullscreen"
        :title="$t('settings.game.windowSize')"
        :description="$t('settings.game.windowSizeDesc')"
        icon="resize">
        <BaseInput
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="$t('settings.game.windowSizeWidth')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.width"
          :lazy-update-value="true">
        </BaseInput>
        <BaseInput
          width="100px"
          style="display: inline-block"
          :placeholder="$t('settings.game.windowSizeHeight')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.height"
          :lazy-update-value="true">
        </BaseInput>
      </SettingItem>
      <SettingItem :title="$t('settings.game.hideLauncherAfterLaunch')" icon="eye-off">
        <BaseSwitch></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.autoRefreshAccount')"
        :description="$t('settings.game.autoRefreshAccountDesc')"
        icon="refresh">
        <BaseSwitch v-model="config.launch.skip_refresh_account"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.autoCompleteGameFiles')"
        :description="$t('settings.game.autoCompleteGameFilesDesc')"
        icon="build">
        <BaseSwitch v-model="config.launch.skip_check_files"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.demo')"
        :description="$t('settings.game.demoDesc')"
        icon="lock">
        <BaseSwitch v-model="config.launch.is_demo"></BaseSwitch>
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

<style lang="less"></style>
