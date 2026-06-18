<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="home-view">
    <div class="current-instance-summary">
      <div class="left-column">
        <img
          :src="currentInstance.config.icon ?? instanceIconFallback"
          alt="Instance icon"
          class="current-instance-icon" />
        <div class="current-instance-info">
          <p class="title">{{ currentInstance.config.name }}</p>
          <p>Some tag here</p>
        </div>
      </div>
      <div class="right-column">
        <button class="open-instance-setting-button">Gear</button>
        <button class="game-button">Launch</button>
      </div>
    </div>
    <div class="current-instance-details"></div>
    <div class="instances-drawer"></div>
  </div>
</template>

<script setup lang="ts">
import { useInstanceStore } from "@/store/instance";
import { computed } from "vue";
import instanceIconFallback from "@/assets/images/Chicken_JE2_BE2.webp";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});
</script>

<style lang="less" scoped>
.home-view {
  width: 100%;
  height: 100%;
  padding: 32px;
  display: flex;
  flex-direction: column;
  .current-instance-summary {
    width: 100%;
    outline: 1px solid #ff000034;
    display: flex;
    justify-content: space-between;
    .left-column {
      outline: 1px solid #ffff0034;
      display: flex;
      .current-instance-icon {
        width: 56px;
        height: 56px;
        border-radius: 10px;
      }
      .current-instance-info {
        .title {
          font-size: 17px;
        }
      }
    }
    .right-column {
      outline: 1px solid #0000ff34;
      display: flex;
    }
  }
  .current-instance-details {
    width: 100%;
    height: 500px;
    outline: 1px solid #00ff0034;
  }
  .current-instances-drawer {
    width: 100%;
    height: 500px;
    outline: 1px solid #0000ff34;
  }
}
</style>
