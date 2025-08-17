<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <setting-group :title="$t('settings.game.jvmTitle')">
      <setting-item
        :title="$t('settings.game.chooseJava')"
        icon="java"
        :icon-size="36"
        :clickAble="true">
        <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{
          $t("settings.game.selectedJava", {
            selected: "Java 17",
          })
        }}</span>
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </setting-item>
      <setting-item
        :title="$t('settings.game.addJava')"
        :description="$t('settings.game.addJavaDesc')"
        icon="cloud-download"
        :clickAble="true">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </setting-item>
      <!-- <setting-item title="Java 内存"></setting-item> -->
    </setting-group>
    <setting-group :title="$t('settings.game.launchOptions')">
      <setting-item
        :title="$t('settings.game.launcherName')"
        :description="$t('settings.game.launcherNameDesc')"
        icon="tag">
        <TextInputBox
          width="360px"
          v-model="config.launch.launcher_name"
          :lazy-update-value="true"></TextInputBox>
      </setting-item>
      <!-- TODO:<setting-item title="服务器地址" description="启动后自动加入服务器" icon="server"> -->
      <!--   <TextInputBox -->
      <!--     width="240px" -->
      <!--     v-model="config.launch.server!.ip" -->
      <!--     style="display: inline-block; margin-right: 16px" -->
      <!--     placeholder="IP 或域名"></TextInputBox> -->
      <!--   <TextInputBox -->
      <!--     width="100px" -->
      <!--     v-model="config.launch.server!.port" -->
      <!--     placeholder="端口" -->
      <!--     style="display: inline-block"></TextInputBox> -->
      <!-- </setting-item> -->
      <setting-item
        :title="$t('settings.game.enterWorldAfterLaunch')"
        :description="$t('settings.game.enterWorldAfterLaunchDesc')"
        icon="enter">
        <TextInputBox
          width="360px"
          :placeholder="$t('settings.game.enterWorldAfterLaunchPlaceholder')"
          :lazy-update-value="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.game.fullscreen')"
        :description="$t('settings.game.fullscreenDesc')"
        icon="expand">
        <ToggleSwitch v-model="config.launch.fullscreen"></ToggleSwitch>
      </setting-item>
      <setting-item
        :disabled="config.launch.fullscreen"
        :title="$t('settings.game.windowSize')"
        :description="$t('settings.game.windowSizeDesc')"
        icon="resize">
        <TextInputBox
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="$t('settings.game.windowSizeWidth')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.width"
          :lazy-update-value="true">
        </TextInputBox>
        <TextInputBox
          width="100px"
          style="display: inline-block"
          :placeholder="$t('settings.game.windowSizeHeight')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.height"
          :lazy-update-value="true">
        </TextInputBox>
      </setting-item>
      <setting-item :title="$t('settings.game.hideLauncherAfterLaunch')" icon="eye-off">
        <toggle-switch></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.autoRefreshAccount')"
        :description="$t('settings.game.autoRefreshAccountDesc')"
        icon="refresh">
        <toggle-switch v-model="config.launch.skip_refresh_account"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.autoCompleteGameFiles')"
        :description="$t('settings.game.autoCompleteGameFilesDesc')"
        icon="build">
        <toggle-switch v-model="config.launch.skip_check_files"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.demo')"
        :description="$t('settings.game.demoDesc')"
        icon="lock">
        <toggle-switch v-model="config.launch.is_demo"></toggle-switch>
      </setting-item>
    </setting-group>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import TextInputBox from "@/components/TextInputBox.vue";
import ToggleSwitch from "@/components/ToggleSwitch.vue";
import { useConfigStore } from "@/store/config";
const config = useConfigStore();
</script>

<style lang="less"></style>
