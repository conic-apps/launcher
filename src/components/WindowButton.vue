<template>
  <div
    class="win-btn win-btn-minimize"
    v-if="buttonType === 'minimize'"
    :style="{ background: appWindowFocused ? 'rgb(254,188,46)' : '#4E4E4E' }"
    @click="$emit(buttonType)"></div>
  <div
    class="win-btn win-btn-maximize"
    v-else-if="buttonType === 'maximize'"
    :style="{ background: appWindowFocused ? 'rgb(97,197,84)' : '#4E4E4E' }"
    @click="$emit(buttonType)"></div>
  <div
    class="win-btn win-btn-close"
    v-else-if="buttonType === 'close'"
    :style="{ background: appWindowFocused ? 'rgb(255,95,87)' : '#4E4E4E' }"
    @click="$emit(buttonType)"></div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { window as appWindow } from "@tauri-apps/api";
import { Event } from "@tauri-apps/api/event";

const props = defineProps<{
  buttonType: "minimize" | "maximize" | "close";
}>();

defineEmits(["minimize", "maximize", "close"]);

const appWindowFocused = ref(true);

appWindow
  .getCurrentWindow()
  .isFocused()
  .then((focused) => {
    appWindowFocused.value = focused;
  });

appWindow.getCurrentWindow().onFocusChanged((event: Event<boolean>) => {
  appWindowFocused.value = event.payload;
});
</script>

<style lang="less" scoped>
div.win-btn {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  margin-left: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  > i {
    font-style: normal;
    font-family: "fa-pro";
    font-weight: 100;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  &:active {
    opacity: 0.8;
  }
}
</style>
