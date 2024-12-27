<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="install-progress-vue">
    <div class="progress-card" v-if="installing">
      <div
        class="step"
        :style="
          getVersionInfoStatus == 'success' || getVersionInfoStatus == 'pending'
            ? 'opacity: 0.8'
            : 'background: rgba(255, 255, 255, 0.08);'
        ">
        <item-loading-icon :status="getVersionInfoStatus"></item-loading-icon>
        <p>获取版本信息</p>
      </div>
      <div
        class="step"
        :style="
          checkExistFilesStatus == 'success' || checkExistFilesStatus == 'pending'
            ? 'opacity: 0.8'
            : 'background: rgba(255, 255, 255, 0.08)'
        ">
        <item-loading-icon :status="checkExistFilesStatus"></item-loading-icon>
        <p>检查已有游戏文件</p>
        <i
          style="font-size: 13px; margin-left: auto; opacity: 0.7"
          v-if="checkExistFilesStatus == 'in-progress'"
          >已检查 {{ tweened.number.toFixed(0) }} 个文件</i
        >
      </div>
      <div
        class="step"
        :style="
          downloadVanillaGameStatus == 'success' || downloadVanillaGameStatus == 'pending'
            ? 'opacity: 0.8'
            : 'background: rgba(255, 255, 255, 0.08)'
        ">
        <item-loading-icon :status="downloadVanillaGameStatus"></item-loading-icon>
        <p>下载原版游戏文件</p>
        <progress-bar
          v-if="installProgress.step == 3"
          width="260"
          style="margin-left: auto"
          :loading="false"
          :value="installProgress.completed.toString()"
          :total="installProgress.total.toString()"></progress-bar>
        <i
          v-if="installProgress.step == 3"
          style="min-width: 146px; text-align: right; font-size: 13px; opacity: 0.7">
          已下载 {{ installProgress.completed }} 个文件，共 {{ installProgress.total }} 个
        </i>
      </div>
      <div
        class="step"
        v-if="!!modLoaderType"
        :style="
          installModLoaderStatus == 'success' || installModLoaderStatus == 'pending'
            ? 'opacity: 0.8'
            : 'background: rgba(255, 255, 255, 0.08)'
        ">
        <item-loading-icon :status="installModLoaderStatus"></item-loading-icon>
        <p>安装 {{ modLoaderType }}</p>
      </div>
    </div>
    <div v-else>安装后才能看实例信息</div>
  </div>
</template>

<script setup lang="ts">
import ItemLoadingIcon from "@/components/ItemLoadingIcon.vue";
import ProgressBar from "@/components/ProgressBar.vue";
import { listen } from "@tauri-apps/api/event";
import { computed, reactive, ref, watch } from "vue";
import type { Ref } from "vue";
import gsap from "gsap";
import { useInstanceStore } from "@/store/instance";

const installing = computed(() => {
  return instanceStore.installProgress.has(instanceStore.currentInstance.config.name);
});

const instanceStore = useInstanceStore();

const modLoaderType = computed(() => {
  return instanceStore.currentInstance.config.runtime.mod_loader_type;
});

let installProgress: Ref<InstallProgress> = ref({
  completed: 0,
  total: 0,
  step: 0,
});
interface InstallProgress {
  completed: number;
  total: number;
  step: number;
}
listen("install_progress", (event) => {
  installProgress.value = event.payload as InstallProgress;
  if (installProgress.value.step != 3) {
    speed.value = "0 B/s";
  }
});
interface InstallError {
  step: number;
  // TODO: error type
}
let installError: Ref<InstallError> = ref({
  step: 0,
});
listen("install_error", (event) => {
  installError.value = event.payload as InstallError;
});
listen("install_success", (_event) => {
  installProgress.value.step = 1000;
});

const getVersionInfoStatus = computed(() => {
  if (installError.value.step == 1) {
    return "error";
  }
  if (installProgress.value.step == 1) {
    return "in-progress";
  }
  if (installProgress.value.step > 1) {
    return "success";
  }
  return "pending";
});
const checkExistFilesStatus = computed(() => {
  if (installError.value.step == 2) {
    return "error";
  }
  if (installProgress.value.step == 2) {
    return "in-progress";
  }
  if (installProgress.value.step > 2) {
    return "success";
  }
  return "pending";
});
const downloadVanillaGameStatus = computed(() => {
  if (installError.value.step == 3) {
    return "error";
    // TODO: show error message
  }
  if (installProgress.value.step == 3) {
    return "in-progress";
  }
  if (installProgress.value.step > 3) {
    return "success";
  }
  return "pending";
});
const installModLoaderStatus = computed(() => {
  if (installError.value.step == 4) {
    return "error";
  }
  if (installProgress.value.step == 4) {
    return "in-progress";
  }
  if (installProgress.value.step > 4) {
    return "success";
  }
  return "pending";
});
let speed = ref("");
listen("download_speed", (event) => {
  let payload = (event.payload as number) / 4;
  if (payload < 1024) {
    speed.value = payload + " B/s";
  } else if (payload < 1024 * 1024) {
    speed.value = (payload / 1024).toFixed(2) + " KB/s";
  } else if (payload < 1024 * 1024 * 1024) {
    speed.value = (payload / 1024 / 1024).toFixed(2) + " MB/s";
  } else {
    speed.value = (payload / 1024 / 1024 / 1024).toFixed(2) + " GB/s";
  }
});
const running = ref(0);
listen("running_download_task", (event) => {
  let payload = event.payload as number;
  running.value = payload;
});
const pending = computed(() => {
  let pending = installProgress.value.total - installProgress.value.completed - running.value;
  if (pending <= 0) {
    return 0;
  } else {
    return pending;
  }
});
const tweened = reactive({
  number: 0,
});

watch(installProgress, (n) => {
  console.log(111);
  gsap.to(tweened, { duration: 0.5, number: Number(n.completed) || 0 });
});
</script>

<style lang="less">
.install-progress-vue {
  width: 100%;
  height: 100%;
}

.install-progress-vue > div {
  width: 100%;
  padding: 10px;
  background: var(--card-background);
  display: flex;
  align-items: center;
  width: 100%;
  border: var(--card-border);
  border-radius: var(--card-border-radius);
  flex-direction: column;
}

.install-progress-vue .progress-card .step {
  height: 36px;
  width: 100%;
  display: flex;
  margin-bottom: 4px;
  border-radius: 6px;
  align-items: center;
  padding: 0px 10px;
}

.install-progress-vue .progress-card .step p {
  margin-left: 10px;
  font-size: 14px;
}
</style>
