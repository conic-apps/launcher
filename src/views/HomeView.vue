<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="home-view">
    <div class="current-instance-summary">
      <div class="left-column">
        <img
          class="current-instance-icon"
          :src="currentInstance.config.icon ?? instanceIconFallback"
          alt="Instance icon" />
        <div class="current-instance-info">
          <p class="title">{{ currentInstance.config.name }}</p>
          <p>Some tag here</p>
        </div>
      </div>
      <div class="right-column">
        <button class="open-instance-setting-button">
          <AppIcon name="settings" :size="24"></AppIcon>
        </button>
        <button
          class="game-button"
          :class="{
            'game-button-launch': currentInstance.installed,
            'game-button-install': !currentInstance.installed,
          }">
          <AppIcon name="play" style="margin-right: 4px" v-if="currentInstance.installed"></AppIcon>
          <AppIcon name="download" style="margin-right: 4px" v-else></AppIcon>
          {{ currentInstance.installed ? "Launch" : "Install" }}
        </button>
      </div>
    </div>
    <!-- <div class="current-instance-details"></div> -->
    <instance-details
      style="margin-top: 16px"
      v-if="instanceStore.currentInstance.installed"></instance-details>
    <div v-else class="instance-details-placeholder">
      <p>To view instance details, please install it first.</p>
    </div>
    <div class="instances-drawer">
      <div
        class="instance"
        v-for="(instance, index) in instanceStore.instances"
        :key="index"
        @click="instanceStore.currentInstance = instance">
        <img :src="instance.config.icon ?? instanceIconFallback" />
        <!-- TODO: Style -->
        <p class="name">{{ instance.config.name }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import InstanceDetails from "./game/InstanceDetails.vue";
import { useInstanceStore } from "@/store/instance";
import { computed } from "vue";
import instanceIconFallback from "@/assets/images/unknown_pack.webp";

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
  align-items: center;

  .current-instance-summary {
    width: 100%;
    display: flex;
    justify-content: space-between;
    background: var(--card-background);
    border: var(--card-border);
    border-radius: var(--card-border-radius);
    padding: 16px;
    transition: all 1s ease;

    .left-column {
      display: flex;

      .current-instance-icon {
        width: 52px;
        height: 52px;
        border-radius: calc(var(--card-icon-border-radius) + 4px);
        background: var(--card-icon-background);
      }
      .current-instance-info {
        margin-left: 16px;

        .title {
          font-size: 22px;
        }
      }
    }
    .right-column {
      display: flex;
      align-items: center;
      .open-instance-setting-button {
        appearance: none;
        background: none;
        border: none;
        width: 32px;
        height: 32px;
        margin-right: 16px;
      }
      .open-instance-setting-button:active {
        opacity: 0.9;
        transform: scale(0.95);
      }
      .game-button {
        appearance: none;
        background: none;
        border: none;
        background: var(--game-install-button-background);
        background-image: linear-gradient(248deg, #235dce, #399bed);
        color: #fff;
        padding: 8px 16px;
        font-size: 18px;
        display: flex;
        align-items: center;
        border-radius: var(--controllers-border-radius);
      }
      .game-button:active {
        opacity: 0.9;
      }
      .game-button-launch {
        background-image: linear-gradient(248deg, #189e47, #41a126);
      }
      .game-button-install {
        background-image: linear-gradient(248deg, #235dce, #399bed);
      }
    }
  }
  .current-instance-details {
    width: 100%;
    margin-top: 24px;
    height: 500px;
  }
  .instance-details-placeholder {
    width: 100%;
    height: calc(100% - 256px);
    border: 1px dashed #888;
    border-radius: var(--card-border-radius);
    margin-top: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    p {
      font-size: 20px;
      opacity: 0.6;
    }
  }
  .instances-drawer {
    position: absolute;
    bottom: 16px;
    left: 24px;
    width: calc(100% - 48px);
    padding: 24px 32px;
    display: flex;
    background: var(--card-background);
    border-top: var(--card-border);
    border-radius: var(--card-border-radius);

    .instance {
      width: 120px;
      height: 100px;
      display: inline-flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      border-radius: var(--card-border-radius);
      border: var(--card-border);
      background: var(--card-background);
      overflow: hidden;
      margin-right: 16px;

      img {
        width: 60px;
        height: 60px;
        border-radius: var(--card-border-radius);
      }
    }
    .instance:active {
      opacity: 0.8;
    }
  }
}
</style>
