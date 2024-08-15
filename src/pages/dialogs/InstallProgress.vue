<template>
  <dialog-vue :visible="props.installing" width="660" height="420">
    <div class="install-progress-vue">
      <div class="title">
        <div style="display: flex; align-items: center">
          <div class="icon">
            <i class="folder-arrow-down"></i>
          </div>
          <div>
            <h4>正在安装"{{ props.instanceName }}"</h4>
            <p style="display: flex; align-items: center">
              <i
                class="stopwatch"
                style="font-family: fa-pro; font-style: normal; margin-right: 0.2em"></i>
              <span style="width: 76px">{{ computedTime }} </span>
              <span
                :style="
                  installProgress.step == 3
                    ? 'transition: all .2s ease'
                    : 'transition: all .2s ease;opacity: 0'
                "
                ><i
                  class="gauge-high"
                  style="font-family: fa-pro; font-style: normal; margin-right: 0.2em"></i>
                {{ speed }}</span
              >
            </p>
          </div>
        </div>
      </div>
      <div class="progress">
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
            >已检查 {{ installProgress.completed }} 个文件</i
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
        <!-- TODO: install mod loader and optifine -->
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import ItemLoadingIcon from "@/components/ItemLoadingIcon.vue";
import ProgressBar from "@/components/ProgressBar.vue";
import { listen } from "@tauri-apps/api/event";
import { computed, ref, watch } from "vue";
import type { Ref } from "vue";

let time = ref(1145141919810);
let timer: number;
const computedTime = computed(() => {
  function doubleNum(n: any) {
    return n < 10 ? `0${n}` : n.toString();
  }
  let second = doubleNum((time.value % 600) / 10);
  let minute = doubleNum(Math.floor(time.value / 600) % 60);
  let hour = doubleNum(Math.floor(time.value / 36000) % 24);
  return `${hour}:${minute}:${second}`;
});

const props = defineProps<{
  installing: boolean;
  instanceName: string;
}>();
watch(props, (val) => {
  if (val.installing == true) {
    time.value = 0;
    timer = setInterval(() => {
      time.value++;
    }, 100);
  }
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
  clearInterval(timer);
});
listen("install_success", (_event) => {
  installProgress.value.step = 1000;
  clearInterval(timer);
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
let speed = ref("");
listen("download_speed", (event) => {
  let payload = (event.payload as number) / 2;
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
</script>

<style lang="less">
.install-progress-vue {
  width: 100%;
  height: 100%;
}

.install-progress-vue .title {
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.install-progress-vue .title h4 {
  font-weight: normal;
  font-size: 22px;
  margin-bottom: 4px;
}

.install-progress-vue .title p {
  font-size: 14px;
  margin-top: 4px;
  opacity: 0.7;
  font-weight: normal;
}

.install-progress-vue .title p i {
  font-size: 13.6px;
}

.install-progress-vue .title .icon {
  width: 80px;
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.install-progress-vue .title .icon i {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.install-progress-vue .title .icon i::before {
  font-size: 32px;
  font-weight: 500;
  font-style: normal;
  font-family: "fa-pro";
}

.install-progress-vue .progress {
  width: 100%;
  padding: 10px;
}

.install-progress-vue .progress .step {
  height: 36px;
  width: 100%;
  display: flex;
  margin-bottom: 4px;
  border-radius: 6px;
  align-items: center;
  padding: 0px 10px;
}

.install-progress-vue .progress .step p {
  margin-left: 10px;
  font-size: 14px;
}
</style>
