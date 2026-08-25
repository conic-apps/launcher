<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="java-settings">
    <ScrollView>
      <p class="wizard-title">Java 虚拟机设置</p>
      <p class="wizard-message" v-if="isSupportedJVMAutoInstallPlatform">
        Conic Launcher 能够极大地简化 Java 环境的配置。启用「自动安装 Java
        运行环境」即可免于手动安装它们！
      </p>
      <p class="wizard-message warn" v-else>
        注意：当前平台可能无法自动安装 Java 运行环境，建议手动安装并禁用「自动安装 Java 运行环境」
      </p>
      <SettingsJVM style="margin-top: 16px"></SettingsJVM>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import ScrollView from "@/components/ScrollView.vue";
import { useConfigStore } from "@/store/config";
import { onMounted } from "vue";
import SettingsJVM from "../settings/SettingsJVM.vue";

const configStore = useConfigStore();

const isSupportedJVMAutoInstallPlatform =
  ((window.__PLATFORM__.arch === "X64" || window.__PLATFORM__.arch === "Aarch64") &&
    (window.__PLATFORM__.os_family === "Windows" || window.__PLATFORM__.os_family === "Macos")) ||
  (window.__PLATFORM__.os_family === "Linux" && window.__PLATFORM__.arch === "X64");

onMounted(() => {
  if (!isSupportedJVMAutoInstallPlatform) {
    configStore.prefer_mojang_java = false;
  }
});
</script>

<style lang="less" scoped>
.java-settings {
  :deep(.setting-group),
  :deep(.setting-collapse) {
    width: 100%;
  }
}
</style>
