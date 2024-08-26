<template>
  <div class="account-manager">
    <card
      v-for="account in config.accounts"
      class="overview"
      :id="account.uuid"
      :title="account.profile_name"
      :description="`UUID: ${account.uuid}`"
      :icon="account.avatar"
      padding="10,12,10,12"
      margin="0,0,10,0"
      :key="account.uuid"></card>
  </div>
</template>

<script setup lang="ts">
import Card from "./Card.vue";
import $ from "jquery";
import { useConfigStore } from "@/config";
const config = useConfigStore();

console.log($("#xxxxx .icon"));
setTimeout(() => {
  $("#xxxxx .icon").attr(
    "style",
    /* css */ `background-image: url(/steve_avatar.webp); width: 36px; height: 36px; margin-right: 10px`,
  );
}, 1);
</script>

<style lang="less" scoped>
div.account-manager .icon {
  background-position: center;
  background-size: contain;
  background-repeat: none;
}
</style>
