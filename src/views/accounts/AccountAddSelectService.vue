<template>
  <div class="account-type-selector-container">
    <p class="description">选择验证服务</p>
    <div class="selection">
      <button class="microsoft item" @click="$emit('select-account-type', 'microsoft')">
        <AppIcon name="logo-microsoft" :size="20"></AppIcon>
        <p>微软</p>
        <AppIcon name="chevron-forward" :size="16"></AppIcon>
      </button>
      <button
        class="authlib-injector item"
        @click="$emit('select-account-type', 'authlib-injector')">
        <AppIcon name="extension-puzzle" :size="20"></AppIcon>
        <p>第三方：Authlib injector</p>
        <AppIcon name="chevron-forward" :size="16"></AppIcon>
      </button>
      <button class="offline item" @click="$emit('select-account-type', 'offline')">
        <AppIcon name="cloud-offline-outline" :size="20"></AppIcon>
        <p>离线</p>
        <AppIcon name="chevron-forward" :size="16"></AppIcon>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseButton from "@/components/base/BaseButton.vue";
defineEmits(["select-account-type"]);
</script>

<style lang="less" scoped>
.account-type-selector-container {
  display: flex;
  width: 100%;
  height: 100%;
  flex-direction: column;

  button.item {
    width: 100%;
    height: 32px;
    padding: 0px 12px;
    display: flex;
    align-items: center;
    justify-content: start;
    border: var(--controllers-border);
    border-radius: var(--controllers-border-radius);
    background: var(--controllers-background);
    margin-top: 8px;
    transition:
      transform 100ms ease,
      opacity 100ms ease;

    p {
      font-size: 14px;
      margin: 0 auto 0 8px;
    }

    &:hover,
    &:focus {
      transform: scale(1.01);
    }

    &:focus {
      outline: var(--controllers-outline-focus);
    }

    &:active {
      opacity: 0.8;
      transform: scale(0.99);
    }
  }

  .description {
    font-size: 14px;
    margin: 8px 8px 0 8px;
  }

  .selection {
    display: flex;
    flex-direction: column;

    .other {
      display: flex;
    }
  }
}
</style>
