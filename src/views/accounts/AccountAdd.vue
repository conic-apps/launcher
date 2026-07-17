<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-add">
    <p class="title">Add Account</p>
    <div class="container" :style="{ width: containerSize.width, height: containerSize.height }">
      <Transition :name="transitionName" mode="out-in">
        <component
          :is="currentComponent"
          @select-account-type="selectAccountType($event)"></component>
      </Transition>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { type Component, computed, markRaw, ref, ShallowRef, shallowRef } from "vue";
import AccountAddMicrosoft from "./AccountAddMicrosoft.vue";
import AccountAddAuthlibInjector from "./AccountAddAuthlibInjector.vue";
import AccountAddOffline from "./AccountAddOffline.vue";
import AccountAddSelectService from "./AccountAddSelectService.vue";

const components = {
  selectAccountService: {
    component: markRaw(AccountAddSelectService),
    containerSize: { width: "300px", height: "180px" },
  },
  addMicrosoftAccount: {
    component: markRaw(AccountAddMicrosoft),
    containerSize: { width: "300px", height: "180px" },
  },
  addAuthlibInjectorAccount: {
    component: markRaw(AccountAddAuthlibInjector),
    containerSize: { width: "300px", height: "180px" },
  },
  addOfflineAccount: {
    component: markRaw(AccountAddOffline),
    containerSize: { width: "300px", height: "180px" },
  },
};
const currentComponent: ShallowRef<Component> = shallowRef(
  components.selectAccountService.component,
);
const containerSize = ref(components.selectAccountService.containerSize);
const transitionName = ref("slide-left");
function selectAccountType(event: "microsoft" | "authlib-injector" | "offline") {
  transitionName.value = "slide-left";
  if (event === "microsoft") {
    currentComponent.value = components.addMicrosoftAccount.component;
  } else if (event === "authlib-injector") {
    currentComponent.value = components.addAuthlibInjectorAccount.component;
  } else if (event === "offline") {
    currentComponent.value = components.addOfflineAccount.component;
  }
}
</script>

<style lang="less" scoped>
.account-add {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;

  div.container {
    display: flex;
    width: 300px;
    height: 180px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: var(--card-border);
    border-radius: var(--card-border-radius);
    background: var(--card-background);
    padding: 10px 20px;
    transition: all 200ms ease;
  }

  p.title {
    font-size: 22px;
    text-align: center;
    width: fit-content;
    margin-bottom: 16px;
  }
}
</style>
