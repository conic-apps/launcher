<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.updateReminder.visible" :width="420" :height="200">
    <div class="main">
      <div class="title">
        <h4>发现新的 Minecraft 版本</h4>
        <p>Minecraft {{ dialogStore.updateReminder.version }} 已发布！</p>
      </div>
      <button class="know" @click="close">知道了</button>
    </div>
  </BaseDialog>
</template>

<script lang="ts" setup>
import BaseDialog from "@/components/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";

const dialogStore = useDialogStore();

function close() {
  dialogStore.updateReminder.visible = false;
}
</script>

<style lang="less" scoped>
.main {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: space-between;

  .title {
    display: flex;
    flex-direction: column;
    align-items: center;

    h4 {
      margin: 0;
    }

    p {
      margin: 12px 0 0;
      font-size: 14px;
    }
  }

  .know {
    appearance: none;
    border: none;
    width: 100%;
    padding: 8px 0;
    border-radius: 4px;
    background: var(--ctp-blue);
    color: var(--ctp-text-inverse);
    transition: transform 200ms ease;

    &:hover {
      transform: scale(1.02);
    }

    &:active {
      transform: scale(0.97);
    }
  }
}
</style>
