<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-add">
    <div class="header">
      <p class="title">选择认证服务并添加帐户</p>
      <AccountAddSelect
        :selectAuthService="selectAuthService"
        v-model:authServiceType="authServiceType"
        :disabled="switchComponentDisabled" />
    </div>
    <Transition :name="transitionName" mode="out-in">
      <component
        :is="currentComponent"
        v-model:switchComponentDisabled="switchComponentDisabled"
        @switch-component-manage="$emit('switch-component-manage')"></component>
    </Transition>
  </div>
</template>

<script lang="ts" setup>
import { type Component, markRaw, ref, shallowRef } from "vue";
import AccountAddMicrosoft from "./AccountAddMicrosoft.vue";
import AccountAddOffline from "./AccountAddOffline.vue";
import AccountAddYggdrasil from "./AccountAddYggdrasil.vue";
import AccountAddSelect from "./AccountAddSelect.vue";

defineEmits(["switch-component-manage"]);

const components = {
  addMicrosoftAccount: {
    component: markRaw(AccountAddMicrosoft),
  },
  addYggdrasilAccount: {
    component: markRaw(AccountAddYggdrasil),
  },
  addOfflineAccount: {
    component: markRaw(AccountAddOffline),
  },
};
const currentComponent = shallowRef<Component>(components.addMicrosoftAccount.component);
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
      break;
    case "yggdrasil":
      currentComponent.value = components.addYggdrasilAccount.component;
      break;
    case "offline":
      currentComponent.value = components.addOfflineAccount.component;
      break;
  }
}

const transitionName = ref<"slide-left" | "slide-right">("slide-left");

const switchComponentDisabled = ref(false);
</script>

<style lang="less" scoped>
.account-add {
  width: 540px;
  display: flex;
  flex-direction: column;
  padding: 8px;

  .header {
    display: flex;
    align-items: center;
    margin-bottom: 16px;

    p.title {
      font-size: 22px;
    }
  }
}
</style>
