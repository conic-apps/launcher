<template>
  <dialog-vue :visible="props.installing" width="660" height="420">
    <div class="install-progress-vue">
      <div class="title">
        <div style="display: flex; align-items: center">
          <div class="icon">
            <i class="folder-arrow-down"></i>
          </div>
          <div>
            <h4>正在安装{{ props.instanceName }}</h4>
            <p>请稍等片刻</p>
          </div>
        </div>
      </div>
      <div class="progress">
        <div class="step" :style="getVersionInfoStatus == 'success' ||
            getVersionInfoStatus == 'pending'
            ? 'opacity: 0.8'
            : 'background: rgba(255, 255, 255, 0.08);'
          ">
          <item-loading-icon :status="getVersionInfoStatus"></item-loading-icon>
          <p>获取版本信息</p>
        </div>
        <div class="step" :style="downloadVanillaGameStatus == 'success' ||
            downloadVanillaGameStatus == 'pending'
            ? 'opacity: 0.8'
            : 'background: rgba(255, 255, 255, 0.08)'
          ">
          <item-loading-icon :status="downloadVanillaGameStatus"></item-loading-icon>
          <p>下载原版游戏文件</p>
          <progress-bar v-if="installProgress.step == 2 || installProgress.step == 3" width="300"
            style="margin-left: auto" :loading="installProgress.step == 2" :value="installProgress.completed.toString()"
            :total="installProgress.total.toString()"></progress-bar>
          <p v-if="installProgress.step == 2 || installProgress.step == 3" style="width: 80px; text-align: right">
            {{ installProgress.completed }} / {{ installProgress.total }}
          </p>
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
import { computed, Ref, ref } from "vue";

const props = defineProps<{
  installing: boolean;
  instanceName: string;
}>();
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
listen("install_success", (event) => {
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
const downloadVanillaGameStatus = computed(() => {
  if (installError.value.step == 2 || installError.value.step == 3) {
    return "error";
    // TODO: show error message
  }
  if (installProgress.value.step == 2 || installProgress.value.step == 3) {
    return "in-progress";
  }
  if (installProgress.value.step > 3) {
    return "success";
  }
  return "pending";
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
}

.install-progress-vue .title p {
  font-size: 14px;
  margin-top: 4px;
  opacity: 0.7;
  font-weight: normal;
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
