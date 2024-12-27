<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <dialog-vue :visible="props.show" :width="width" :height="height">
    <div class="account-manager">
      <div style="width: 100%; height: 100%">
        <div class="title">
          <div style="display: flex; align-items: center">
            <div class="icon">
              <i class="user"></i>
            </div>
            <div>
              <h4>管理游戏帐号</h4>
              <p>添加、删除或选择要使用的游戏帐号</p>
            </div>
          </div>
          <div class="button" style="position: absolute; right: 0" @click="$emit('close')">
            <i></i>
          </div>
        </div>

        <div class="content">
          <Transition :name="transitionName" mode="out-in">
            <component :is="currentComponent" @add="addAccount"> </component>
          </Transition>
        </div>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import { markRaw, reactive, ref, shallowRef } from "vue";
import DialogVue from "@/components/Dialog.vue";
import View from "./account/View.vue";
import Add from "./account/Add.vue";

const props = defineProps<{
  show: boolean;
}>();
const emit = defineEmits(["close", "update", "choose-account"]);

const width = ref(720);
const height = ref(420);

const pages = reactive({
  view: markRaw(View),
  add: markRaw(Add),
});
const currentComponent: any = shallowRef(pages.view);
const transitionName = ref("slide-left");

function addAccount() {
  transitionName.value = "slide-left";
  currentComponent.value = pages.add;

  setTimeout(() => {
    width.value = 400;
    height.value = 400;
  }, 100);
}
</script>

<style lang="less" scoped>
.account-manager {
  width: 100%;
  height: 100%;

  overflow: hidden;

  .title {
    border-bottom: 1px solid #ffffff18;
    width: 100%;
    height: 80px;
    display: flex;
    justify-content: space-between;
    position: relative;

    .icon {
      width: 80px;
      height: 80px;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    i {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    i::before {
      font-size: 36px;
      font-style: normal;
      font-family: "fa-pro";
    }

    h4 {
      font-size: 22px;
      font-weight: normal;
    }

    p {
      font-size: 14px;
      margin-top: 4px;
      opacity: 0.7;
      font-weight: normal;
    }
  }

  .button {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    margin-left: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 100ms;
    background: var(--close-btn-background);

    i::before {
      content: "\f00d";
      font-size: 12px;
      margin-top: 1px;
      margin-left: 0.6px;
      font-style: normal;
      font-family: "fa-pro";
      opacity: 0;
      transition: all 70ms ease;
    }

    i {
      transition: all 100ms ease;
    }
  }

  .button:hover i::before {
    opacity: 1;
  }

  .button:active i {
    opacity: 0.7;
  }

  .content {
    display: flex;
    height: calc(100% - 80px);
    padding-top: 16px;
  }
}
</style>
