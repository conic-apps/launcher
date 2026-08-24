<template>
  <div class="account-add-selection">
    <button
      class="microsoft item"
      :class="{ active: authServiceType === 'microsoft' }"
      @click="selectAuthService('microsoft')">
      <AppIcon name="logo-microsoft" :size="15"></AppIcon>
      <p>微软</p>
    </button>
    <button
      class="yggdrasil item"
      :class="{ active: authServiceType === 'yggdrasil' }"
      @click="selectAuthService('yggdrasil')">
      <AppIcon name="extension-puzzle" :size="15"></AppIcon>
      <p>外置</p>
    </button>
    <button
      class="offline item"
      :class="{ active: authServiceType === 'offline' }"
      @click="selectAuthService('offline')">
      <AppIcon name="cloud-offline-outline" :size="15"></AppIcon>
      <p>离线</p>
    </button>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  selectAuthService: (service: "microsoft" | "yggdrasil" | "offline") => void;
}>();
const authServiceType = defineModel<"microsoft" | "offline" | "yggdrasil">("authServiceType", {
  required: true,
});
</script>

<style lang="less" scoped>
.account-add-selection {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;

  .item {
    appearance: none;
    border: none;
    background: none;
    height: 32px;
    padding: 0 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    font-size: 13px;
    color: rgba(var(--ctp-text-rgb), 0.7);
    transition: background 100ms ease;

    p {
      margin-left: 4px;
    }

    &:hover {
      background: var(--ctp-surface0);
    }

    &:active {
      background: var(--ctp-surface1);
    }
  }

  .item.active {
    background: var(--ctp-surface2);
    color: rgba(var(--ctp-text-rgb), 1);
  }
}
</style>
