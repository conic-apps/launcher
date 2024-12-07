<template>
  <div>
    <div class="info sponsor">
      <h2 style="margin-bottom: 8px">{{ $t("settings.about.sponsorTitle") }}</h2>
      {{ $t("settings.about.sponsorDesc") }}
    </div>
    <setting-group>
      <setting-item
        :title="$t('settings.about.report')"
        :description="$t('settings.about.reportDesc')"
        icon="bug"
        :clickAble="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
    <SettingGroup :title="$t('settings.about.thanks')"> </SettingGroup>
    <SettingGroup :title="$t('settings.about.thirdPartyLibraries')">
      <setting-item
        title="Tauri"
        description="Copyright 2022 Tauri Programme within The Commons Conservancy<br> Licensed under the Apache-2.0 License or MIT Licenses"
        icon="tauri"></setting-item>
      <setting-item
        title="Vue"
        description="Copyright (c) 2018-present, Yuxi (Evan) You and Vue contributors<br> Licensed under the MIT License"
        icon="vuejs"></setting-item>
      <setting-item
        title="jQuery"
        description="Copyright 2024 OpenJS Foundation and jQuery contributors.<br> Licensed under the MIT License"
        icon="jquery"></setting-item>
      <setting-item
        title="vite"
        description="Copyright (c) 2019-present, Yuxi (Evan) You and Vite contributors<br> Licensed under the MIT License"
        icon="vite"></setting-item>
      <setting-item
        title="Less"
        description="Copyright (c) 2009-2017 Alexis Sellier & The Core Less Team<br> Licensed under the Apache License."
        icon="less"></setting-item>
      <setting-item
        title="Tokio"
        description="Copyright (c) 2019 The Tokio Project Developers<br> Licensed under the MIT License."
        icon="tokio"></setting-item>
      <setting-item
        title="Serde"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="Anyhow"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="OneCell"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="Rejex"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
      <setting-item
        title="Rayon"
        description="Rayon is distributed under the terms of both the MIT license and the Apache License (Version 2.0)."></setting-item>
      <setting-item
        title="env_logger"
        description="Licensed under Apache-2.0 or MIT license."></setting-item>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
</script>

<style lang="less" scoped>
.info {
  width: calc(100% - 16px);
  margin: 0 auto;
  margin-bottom: 16px;
  padding: 20px;
  border-radius: 10px;
  line-height: 1.6;
  background: #ffffff18;
  font-size: 14px;
  transition: all 0.2s ease;
}

.sponsor {
  background: url(@/assets/images/bg-1.jpg);
  background-size: cover;
  background-repeat: no-repeat;
  border: 1px solid #ffffff18;
  cursor: pointer;

  * {
    cursor: pointer;
  }
}

.sponsor:hover {
  transform: scale(1.01);
}

.sponsor:active {
  transform: scale(0.98);
  opacity: 0.9;
}
</style>
