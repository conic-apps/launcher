<template>
  <div
    class="win-btn"
    v-if="buttonType === 'minimize'"
    style="background: var(--min-btn-background)"
    @click="$emit(buttonType)">
    <AppIcon name="minus" :size="16"></AppIcon>
  </div>
  <div
    class="win-btn"
    v-else-if="buttonType === 'maximize'"
    style="background: var(--max-btn-background)"
    @click="$emit(buttonType)">
    <AppIcon name="expand-2" :size="16"></AppIcon>
  </div>
  <div
    class="win-btn"
    v-else-if="buttonType === 'close'"
    style="background: var(--close-btn-background)"
    @click="$emit(buttonType)">
    <AppIcon name="xmark" :size="16"></AppIcon>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  buttonType: "minimize" | "maximize" | "close";
}>();

defineEmits(["minimize", "maximize", "close"]);
</script>

<style lang="less" scoped>
div.win-btn {
  width: 20px;
  height: 20px;
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

  svg {
    opacity: 0;
  }
}

div.win-btn:hover > svg {
  opacity: 1;
  fill: var(--window-btn-icon-color);
}

div.win-btn:active {
  transform: scale(0.9);
  > svg {
    opacity: 0.8;
  }
}
</style>
