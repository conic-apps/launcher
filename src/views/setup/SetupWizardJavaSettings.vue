<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="java-settings">
    <ScrollView>
      <p class="wizard-title">{{ t("setup.javaSettings.title") }}</p>
      <p class="wizard-message" v-if="isSupportedJVMAutoInstallPlatform">
        {{ t("setup.javaSettings.desc") }}
      </p>
      <p class="wizard-message warn" v-else>
        {{ t("setup.javaSettings.note") }}
      </p>
      <SettingsJVM style="margin-top: 16px"></SettingsJVM>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import ScrollView from "@/components/ScrollView.vue";
import { useConfigStore } from "@/store/config";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
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
