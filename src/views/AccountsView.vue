<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="accounts-view">
    <Transition name="fade" mode="out-in">
      <component
        :is="currentComponent"
        @switch-component-add="currentComponent = components.add"
        @switch-component-manage="currentComponent = components.manage"></component>
    </Transition>
    <Transition name="custom-slide-bottom">
      <div class="back-button" v-if="hasAccounts && currentComponent === components.add">
        <button @click="currentComponent = components.manage">
          <div>
            <AppIcon name="arrow-back-outline"></AppIcon>
          </div>
          <span>返回档案列表</span>
        </button>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { type Component, computed, markRaw, ShallowRef, shallowRef, watch } from "vue";
import AccountAdd from "./accounts/AccountAdd.vue";
import { useAccountStore } from "@/store/account";
import AccountManager from "./accounts/AccountManager.vue";
import AppIcon from "@/components/AppIcon.vue";

const components = {
  manage: markRaw(AccountManager),
  add: markRaw(AccountAdd),
};

const accounts = useAccountStore();
const hasAccounts = computed(
  () =>
    Object.keys(accounts.yggdrasil).length > 0 ||
    accounts.microsoft.length > 0 ||
    accounts.offline.length > 0,
);
watch(hasAccounts, (value, oldValue) => {
  if (value && !oldValue) {
    currentComponent.value = components.manage;
  } else if (!value && oldValue) {
    currentComponent.value = components.add;
  }
});
const currentComponent: ShallowRef<Component> = hasAccounts.value
  ? shallowRef(components.manage)
  : shallowRef(components.add);
</script>

<style lang="less" scoped>
.accounts-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  .back-button {
    position: absolute;
    bottom: 16px;
    left: 16px;
    button {
      appearance: none;
      background: var(--ctp-latte-lavender);
      border: none;
      width: 120px;
      height: 40px;
      border-radius: 1000px;
      display: flex;
      align-items: center;
      transition: all 0.3s ease;
      transform: scale(1);
      div {
        background: #ffffff3f;
        border-radius: 100px;
        width: 30px;
        height: 30px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-left: 5px;
      }
      span {
        margin-left: 4px;
      }
      &:active {
        transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
        transform: scale(0.97);
      }
    }
  }
}

.custom-slide-bottom-leave-active {
  transition: all 0.3s cubic-bezier(0.75, 0, 1, 0.2);
}

.custom-slide-bottom-enter-active {
  transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
}

.custom-slide-bottom-leave-from {
  transform: translate(0, 0);
}

.custom-slide-bottom-leave-to {
  transform: translate(0, 70px);
}

.custom-slide-bottom-enter-from {
  transform: translate(0, 70px);
}

.custom-slide-bottom-enter-to {
  transform: translate(0, 0);
}
</style>
