<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup>
      <SettingItem
        :title="'让 Conic Launcher 自动管理 Java 运行环境'"
        :description="'自动选择和管理 Java 运行环境，当前版本暂不支持自定义 Java 运行环境'"
        disabled>
        <BaseSwitch v-model="autoSelectJava"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { ref } from "vue";

const autoSelectJava = ref(true);
</script>

<style lang="less"></style>
