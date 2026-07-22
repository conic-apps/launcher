<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="$t('settings.game.jvmTitle')">
      <SettingItem
        :title="$t('settings.game.autoSelectJava')"
        :description="$t('settings.game.autoSelectJavaDesc')"
        icon="refresh">
        <BaseSwitch v-model="autoSelectJava"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.selectDefaultJava')"
        icon="java"
        :icon-size="36"
        :navigable="true">
        <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{
          $t("settings.game.selectedJava", {
            selected: "Java 17",
          })
        }}</span>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import { ref } from "vue";

const autoSelectJava = ref(true);
</script>

<style lang="less"></style>
