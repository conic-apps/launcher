<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.noAccountError.visible" :width="500" :height="146">
    <div class="confirm-quit-app" ref="main">
      <div style="display: flex; align-items: center">
        <AppIcon name="warning" :size="50"></AppIcon>
        <div class="message">
          <p style="font-size: 16px">要启动游戏，你必须至少已添加一个帐户</p>
        </div>
      </div>
      <div class="buttons">
        <BaseButton
          class="back"
          @click="
            dialogStore.noAccountError.visible = false;
            navigationStore.back();
          "
          >取消启动</BaseButton
        >
        <BaseButton
          class="quit"
          @click="
            dialogStore.noAccountError.visible = false;
            navigationStore.navigate('accounts');
          "
          >添加帐户</BaseButton
        >
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import BaseDialog from "@/components/base/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";
import { useNavigationStore } from "@/store/navigation";

const dialogStore = useDialogStore();
const navigationStore = useNavigationStore();
</script>

<style lang="less" scoped>
.confirm-quit-app {
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
    button.back {
      margin-right: 8px;
      padding: 8px 0;
    }
    button.quit {
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
    }
  }
}
</style>
