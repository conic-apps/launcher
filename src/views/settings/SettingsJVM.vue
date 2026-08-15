<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup>
      <SettingItem
        :title="'优先使用 Mojang 官方提供的 Java 运行环境'"
        :description="'安装游戏时一并安装 Java 运行环境，启动时优先从启动器目录中查找。关闭后将仅尝试使用系统中已安装的 Java 启动游戏'">
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
          :title="formatJavaTitle(runtime)"
          :description="formatJavaDescription(runtime)">
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

function formatJavaTitle(runtime: JavaRuntime): string {
  return `Java ${runtime.major_version} · ${VENDOR_DISPLAY[runtime.vendor]} · ${runtime.version}`;
}

function formatJavaDescription(runtime: JavaRuntime): string {
  const parts: string[] = [];
  if (runtime.is_jdk) parts.push("JDK");
  if (runtime.arch !== "unknown") parts.push(runtime.arch);
  parts.push(runtime.is_valid ? runtime.path : `${runtime.path}（无效）`);
  return parts.join(" · ");
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

<style lang="less"></style>
