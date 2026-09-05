<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.updateReminder.visible" :width="400">
    <div class="update-reminder" ref="main">
      <div class="title">
        <div class="version-icon">
          <img
            v-if="dialogStore.updateReminder.versionType === 'release'"
            style="width: 100%; height: 100%; opacity: 0.8"
            src="@/assets/images/minecraft.webp"
            alt="" />
          <img
            v-else
            style="width: 100%; height: 100%"
            src="@/assets/images/Command_Block.webp"
            alt="" />
        </div>
        <div class="message">
          <p style="font-size: 16px">Minecraft 1.18.2 已发布</p>
          <p style="font-size: 16px"></p>
        </div>
      </div>
      <div class="buttons">
        <BaseButton class="ok" @click="dialogStore.updateReminder.visible = false">好</BaseButton>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/BaseDialog.vue";
import BaseButton from "@/components/BaseButton.vue";
import { useDialogStore } from "@/store/dialog";

const dialogStore = useDialogStore();
</script>

<style lang="less" scoped>
.update-reminder {
  padding: 8px;
  div.title {
    display: flex;
    div.version-icon {
      width: 64px;
    }
  }
  .message {
    display: flex;
    flex-direction: column;
    justify-content: center;
    text-align: center;
  }
  .buttons {
    display: flex;
    width: 100%;
    margin-top: 16px;
    gap: 8px;

    button {
      appearance: none;
      border: none;
      width: 100%;
      border-radius: 4px;
      transition: transform 200ms ease;
    }
    button.ok {
      background: var(--ctp-blue);
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
