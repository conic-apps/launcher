<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="accounts-view">
    <Transition name="entrance" mode="out-in">
      <component
        :is="currentComponent"
        :accounts="accounts"
        @switch-component-add="currentComponent = components.add"
        @switch-component-back="currentComponent = components.list"></component>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { type Component, computed, markRaw, ShallowRef, shallowRef, watch } from "vue";
import AccountList from "./accounts/AccountList.vue";
import AccountAdd from "./accounts/AccountAdd.vue";
import { useAccountStore } from "@/store/account";

const components = {
  list: markRaw(AccountList),
  add: markRaw(AccountAdd),
};

const accounts = useAccountStore();
const hasAccounts = computed(() => {
  console.log(accounts.third_party_yggdrasil);
  console.log(accounts.microsoft);
  console.log(accounts.offline);
  return (
    accounts.third_party_yggdrasil.length || accounts.microsoft.length || accounts.offline.length
  );
});
watch(hasAccounts, (value, oldValue) => {
  if (value && !oldValue) {
    currentComponent.value = components.list;
  } else if (!value && oldValue) {
    currentComponent.value = components.add;
  }
});
const currentComponent: ShallowRef<Component> = hasAccounts.value
  ? shallowRef(components.list)
  : shallowRef(components.add);
</script>

<style lang="less" scoped>
.accounts-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}
</style>
