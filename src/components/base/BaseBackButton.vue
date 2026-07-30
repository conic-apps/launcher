<template>
  <div class="back-button">
    <button>
      <AppIcon name="arrow-back-outline"></AppIcon>
    </button>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
</script>

<style lang="less" scoped>
.back-button {
  position: fixed;
  top: 32px;
  left: 16px;
  button {
    appearance: none;
    background: var(--ctp-latte-lavender);
    border: none;
    border-radius: 1000px;
    display: flex;
    align-items: center;
    transition: all 0.3s ease;
    transform: scale(1);
    width: 32px;
    height: 32px;
    justify-content: center;
    &:active {
      transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
      transform: scale(0.97);
    }
  }
}
</style>
