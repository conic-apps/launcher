<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="accounts-view">
    <AccountManager @switch-component-add="openAdd"></AccountManager>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useDialogStore } from "@/store/dialog";
import { useAccountStore } from "@/store/account";
import AccountManager from "./accounts/AccountManager.vue";

const accounts = useAccountStore();
const dialogStore = useDialogStore();

function openAdd() {
  dialogStore.accountAdd.visible = true;
}

const hasAccounts = computed(
  () =>
    Object.keys(accounts.yggdrasil).length > 0 ||
    accounts.microsoft.length > 0 ||
    accounts.offline.length > 0,
);

onMounted(() => {
  if (!hasAccounts.value) {
    openAdd();
  }
});

watch(hasAccounts, (value, oldValue) => {
  if (!value && oldValue) {
    openAdd();
  }
});
</script>

<style lang="less" scoped>
.accounts-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}
</style>
