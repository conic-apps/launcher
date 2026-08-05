<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.confirmQuitApp.visible" :width="500" :height="144">
    <div class="confirm-quit-app" ref="main">
      <div style="display: flex; align-items: center">
        <AppIcon name="warning" :size="50"></AppIcon>
        <div class="message">
          <p style="font-size: 17px">正在进行的任务将被中止，你确定要退出吗？</p>
          <p style="font-size: 12px; margin-top: 8px">最后的反悔机会</p>
        </div>
      </div>
      <div class="buttons">
        <button class="back" @click="dialogStore.confirmQuitApp.visible = false">
          稍等一下...
        </button>
        <button class="quit" @click="closeWindow">让我出去！</button>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseDialog from "@/components/base/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";
import { window as appWindow } from "@tauri-apps/api";

const dialogStore = useDialogStore();

function closeWindow() {
  requestAnimationFrame(() => {
    document.body.style.transition = "all 250ms cubic-bezier(0, 0.74, 0.65, 1)";
    document.body.style.transform = "scale(0.93)";
    document.body.style.opacity = "0";
    setTimeout(() => {
      appWindow.getCurrentWindow().close();
    }, 500);
  });
}
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
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
      padding: 8px 0;
    }
    button.quit {
      background: var(--ctp-red);
      color: var(--ctp-text-inverse);
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
