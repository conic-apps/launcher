<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <dialog-vue :visible="dialogStore.updateReminder.visible" :width="420" :height="200">
    <div class="main">
      <div class="title">
        <h4>发现新的 Minecraft 版本</h4>
        <p>Minecraft {{ latest }} 已发布！</p>
      </div>
    </div>
  </dialog-vue>
</template>

<script lang="ts" setup>
import DialogVue from "@/components/Dialog.vue";
import { useDialogStore } from "@/store/dialog";
import { ref } from "vue";

const latest = ref("");
const dialogStore = useDialogStore();

// TODO: check new version api, invoke it
</script>

<style lang="less"></style>
