<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.noSuitableJavaError.visible" :width="500">
    <div class="no-suitable-java-error" ref="main">
      <div style="display: flex; align-items: center">
        <AppIcon name="warning" :size="50"></AppIcon>
        <div class="message">
          <p style="font-size: 17px">无法找到最合适的 Java 运行环境</p>
          <p style="font-size: 12px; margin-top: 8px">你可以在实例设置中手动指定一个</p>
        </div>
      </div>
      <div class="buttons">
        <button class="cancel" @click="cancel">取消启动</button>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseDialog from "@/components/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";
import { useNavigationStore } from "@/store/navigation";

const dialogStore = useDialogStore();
const navigationStore = useNavigationStore();

function cancel() {
  dialogStore.noSuitableJavaError.visible = false;
  navigationStore.back();
}
</script>

<style lang="less" scoped>
.no-suitable-java-error {
  padding: 8px;
  .message {
    display: flex;
    flex-direction: column;
    justify-content: center;
    margin-left: 16px;
  }
  .buttons {
    display: flex;
    width: 100%;
    margin-top: 16px;
    button {
      appearance: none;
      border: none;
      width: 100%;
      border-radius: 4px;
      transition: transform 200ms ease;
    }
    button.cancel {
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
      padding: 8px 0;
    }
    button:hover {
      transform: scale(1.02);
    }
    button:active {
      transform: scale(0.97);
    }
  }
}
</style>
