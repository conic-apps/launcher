<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="accounts-view">
    <Transition name="entrance" mode="out-in">
      <component
        :is="currentComponent"
        @switch-component-add="currentComponent = components.add"
        @switch-component-back="currentComponent = components.list"></component>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { type Component, computed, markRaw, Ref, ref, ShallowRef, shallowRef, watch } from "vue";
import AccountList from "./accounts/AccountList.vue";
import AccountAdd from "./accounts/AccountAdd.vue";
import { Accounts } from "@conic/account";
import { useConfigStore } from "@/store/config";

const components = {
  list: markRaw(AccountList),
  add: markRaw(AccountAdd),
};

const accounts: Ref<Accounts | null> = ref(null);
const configStore = useConfigStore();
const hasAccounts = computed(() => {
  return (
    accounts.value &&
    (accounts.value.authlib_injector || accounts.value.microsoft || accounts.value.offline)
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
