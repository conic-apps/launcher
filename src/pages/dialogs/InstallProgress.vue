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
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from '@/components/Dialog.vue';

const props = defineProps<{
  installing: boolean,
  instanceName: string
}>()
</script>

<style lang="less">
.install-progress-vue {
  width: 100%;
  height: 100%;
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
</style>