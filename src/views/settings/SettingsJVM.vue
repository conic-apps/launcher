<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="setting-jvm">
    <SettingGroup>
      <SettingItem
        :title="'优先使用 Mojang 提供的 Java 运行环境'"
        :description="'此选项有助于避免个别平台（例如 macOS ）的奇怪问题，如果你的 Java 环境出现问题或者懒得自己安装 Java ，直接打开此选项即可。关闭后将仅尝试使用系统中已安装的 Java 启动游戏'">
        <BaseSwitch v-model="config.prefer_mojang_java"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingCollapse
      :title="'管理已安装的 Java 运行环境'"
      :description="'关闭某个 Java 后，启动器将不会使用它来启动游戏'">
      <SettingItem v-if="loading" :title="'正在扫描已安装的 Java 运行环境'">
        <ItemLoadingIcon status="in-progress"></ItemLoadingIcon>
      </SettingItem>
      <SettingItem v-else-if="scanError" :title="'扫描失败'" :description="scanError"></SettingItem>
      <template v-else>
        <SettingItem
          v-if="runtimes.length === 0"
          :title="'未检测到已安装的 Java 运行环境'"
          :description="'安装游戏时 Conic Launcher 会自动从 Mojang 服务器下载所需的 Java 运行环境'"></SettingItem>
        <SettingItem
          v-for="runtime in runtimes"
          :key="runtime.path"
          :description="formatJavaPath(runtime.path)">
          <template #title>
            <p style="font-size: 13px; display: flex; gap: 4px; align-items: center">
              <span style="margin-right: 4px">Java {{ runtime.major_version }}</span>
              <span class="vendor tag"> {{ VENDOR_DISPLAY[runtime.vendor] }} </span>
              <span class="version tag"> {{ runtime.version }} </span>
              <span class="version tag"> {{ runtime.arch }} </span>
            </p>
          </template>
          <BaseSwitch
            :model-value="isJavaEnabled(runtime)"
            @update:model-value="setJavaEnabled(runtime, $event)"></BaseSwitch>
        </SettingItem>
      </template>
    </SettingCollapse>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import SettingCollapse from "@/components/SettingCollapse.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import ItemLoadingIcon from "@/components/ItemLoadingIcon.vue";
import { useConfigStore } from "@/store/config";
import { scanJava, type JavaRuntime, type JavaVendor } from "@conic/java-runtime";
import { onMounted, ref } from "vue";

const config = useConfigStore();

const runtimes = ref<JavaRuntime[]>([]);
const loading = ref(false);
const scanError = ref<string | null>(null);

const VENDOR_DISPLAY: Record<JavaVendor, string> = {
  oracle: "Oracle",
  openjdk: "OpenJDK",
  eclipse_adoptium: "Eclipse Adoptium",
  microsoft: "Microsoft",
  amazon_corretto: "Amazon Corretto",
  azul_zulu: "Azul Zulu",
  bellsoft_liberica: "BellSoft Liberica",
  semeru: "IBM Semeru",
  sap: "SAP",
  dragonwell: "Alibaba Dragonwell",
  unknown: "Unknown",
};

function formatJavaPath(path: string): string {
  if (path.startsWith("\\\\?\\UNC\\")) {
    return "\\\\" + path.slice(8);
  }
  if (path.startsWith("\\\\?\\")) {
    return path.slice(4);
  }
  return path;
}

function isJavaEnabled(runtime: JavaRuntime): boolean {
  return !config.disabled_java_runtime.includes(runtime.path);
}

function setJavaEnabled(runtime: JavaRuntime, enabled: boolean) {
  const disabled = [...config.disabled_java_runtime];
  const index = disabled.indexOf(runtime.path);
  if (enabled && index !== -1) {
    disabled.splice(index, 1);
  } else if (!enabled && index === -1) {
    disabled.push(runtime.path);
  }
  config.disabled_java_runtime = disabled;
}

onMounted(async () => {
  loading.value = true;
  try {
    const result = await scanJava();
    runtimes.value = result.runtimes.filter((runtime) => !runtime.is_managed);
  } catch (error) {
    scanError.value = String(error);
  } finally {
    loading.value = false;
  }
});
</script>

<style lang="less">
.setting-jvm {
  .tag {
    border: 1px solid var(--ctp-blue);
    padding: 2px 6px;
    border-radius: 100px;
    font-size: 11px;
  }
}
</style>
