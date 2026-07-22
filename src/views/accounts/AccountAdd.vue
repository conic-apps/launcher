<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-add">
    <p class="title">选择认证服务并添加帐户</p>
    <div class="container" :style="{ width: containerSize.width, height: containerSize.height }">
      <div class="selection">
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
      <Transition :name="transitionName" mode="out-in">
        <component
          :is="currentComponent"
          @switch-component-manage="$emit('switch-component-manage')"></component>
      </Transition>
    </div>
  </div>
</template>

<script lang="ts" setup>
import { type Component, markRaw, ref, shallowRef } from "vue";
import AccountAddMicrosoft from "./AccountAddMicrosoft.vue";
import AccountAddOffline from "./AccountAddOffline.vue";
import AccountAddYggdrasil from "./AccountAddYggdrasil.vue";

defineEmits(["switch-component-manage"]);

const components = {
  addMicrosoftAccount: {
    component: markRaw(AccountAddMicrosoft),
    containerSize: { width: "500px", height: "210px" },
  },
  addYggdrasilAccount: {
    component: markRaw(AccountAddYggdrasil),
    containerSize: { width: "500px", height: "280px" },
  },
  addOfflineAccount: {
    component: markRaw(AccountAddOffline),
    containerSize: { width: "500px", height: "264px" },
  },
};
const currentComponent = shallowRef<Component>(components.addMicrosoftAccount.component);
const containerSize = ref(components.addMicrosoftAccount.containerSize);
const authServiceType = ref<"microsoft" | "yggdrasil" | "offline">("microsoft");
function selectAuthService(service: "microsoft" | "yggdrasil" | "offline") {
  if (service === authServiceType.value) {
    return;
  }
  if (authServiceType.value === "microsoft") {
    transitionName.value = "slide-left";
  } else if (authServiceType.value === "offline") {
    transitionName.value = "slide-right";
  } else {
    if (service === "microsoft") {
      transitionName.value = "slide-right";
    } else {
      transitionName.value = "slide-left";
    }
  }
  authServiceType.value = service;
  switch (service) {
    case "microsoft":
      currentComponent.value = components.addMicrosoftAccount.component;
      containerSize.value = components.addMicrosoftAccount.containerSize;
      break;
    case "yggdrasil":
      currentComponent.value = components.addYggdrasilAccount.component;
      containerSize.value = components.addYggdrasilAccount.containerSize;
      break;
    case "offline":
      currentComponent.value = components.addOfflineAccount.component;
      containerSize.value = components.addOfflineAccount.containerSize;
      break;
  }
}

const transitionName = ref<"slide-left" | "slide-right">("slide-left");
</script>

<style lang="less" scoped>
.account-add {
  margin: auto;
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
    justify-content: initial;
    border: var(--card-border);
    border-radius: var(--card-border-radius);
    background: var(--card-background);
    padding: 20px 20px;
    transition: all 200ms ease;
    overflow: hidden;
  }

  p.title {
    font-size: 22px;
    text-align: center;
    width: fit-content;
    margin-bottom: 16px;
  }

  .selection {
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
}
</style>
