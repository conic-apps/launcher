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
  flex-direction: 0;
  margin-bottom: 16px;

  .item {
    appearance: none;
    border: none;
    border-radius: 100px;
    background: #ffffff0f;
    width: 88px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin: 0 4px;
    transition: background 100ms ease;

    p {
      margin-left: 4px;
      font-size: 13px;
    }
  }

  .item.active {
    border: 1px solid #ffffff3f;
    background: #ffffff3f;
  }
}
</style>
