<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="import-instances">
    <p class="wizard-title">导入实例</p>
    <p class="wizard-message">你可以从其他启动器导入先前游玩的实例以便快速开始游戏</p>
    <div style="width: 100%; display: flex; justify-content: center; margin: 12px 0">
      <button class="import-from-other-launcher">导入实例</button>
    </div>
    <p class="wizard-message">
      如果你第一次接触 Minecraft，你也可以选择以最新 Minecraft 版本立刻创建空白实例
    </p>
    <div style="width: 100%; display: flex; justify-content: center; gap: 16px; margin: 12px 0">
      <button
        class="create-latest-instance"
        :class="{ creating: creatingLatestRelease, error: createLatestReleaseErrorOccured }"
        @click="createLatestReleaseInstance">
        {{ createLatestReleaseButtonText }}
      </button>
      <button
        class="create-latest-instance"
        :class="{
          creating: creatingLatestSnapshot,
          error: createLatestSnapshotErrorOccured,
        }"
        @click="createLatestSnapshotInstance">
        {{ createLatestSnapshotButtonText }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { createInstance } from "@conic/instance";
import { getMinecrafVersionManifest } from "@conic/install";
import { computed, ref } from "vue";

const createdLatestRelease = ref(null as null | string);
const createdLatestSnapshot = ref(null as null | string);
const creatingLatestRelease = ref(false);
const creatingLatestSnapshot = ref(false);
const createLatestReleaseErrorOccured = ref(false);
const createLatestSnapshotErrorOccured = ref(false);

async function createLatestReleaseInstance() {
  creatingLatestRelease.value = true;
  try {
    const minecraftVersionManifest = await getMinecrafVersionManifest();
    await createInstance({
      launch_config: { enable_instance_specific_settings: false },
      name: "最新版本",
      runtime: { minecraft: minecraftVersionManifest.latest.release },
    });
    createdLatestRelease.value = minecraftVersionManifest.latest.release;
  } catch (error) {
    console.log("Failed to create latest release instance", error);
    createLatestReleaseErrorOccured.value = true;
  } finally {
    creatingLatestRelease.value = false;
  }
}

async function createLatestSnapshotInstance() {
  creatingLatestSnapshot.value = true;
  try {
    const minecraftVersionManifest = await getMinecrafVersionManifest();
    await createInstance({
      launch_config: { enable_instance_specific_settings: false },
      name: "最新快照",
      runtime: { minecraft: minecraftVersionManifest.latest.snapshot },
    });
    createdLatestSnapshot.value = minecraftVersionManifest.latest.snapshot;
  } catch (error) {
    console.log("Failed to create latest snapshot instance", error);
    createLatestSnapshotErrorOccured.value = true;
  } finally {
    creatingLatestSnapshot.value = false;
  }
}

const createLatestReleaseButtonText = computed(() => {
  if (createLatestReleaseErrorOccured.value) {
    return "出现错误";
  } else if (creatingLatestRelease.value) {
    return "正在创建...";
  } else {
    return createdLatestRelease.value ?? "以最新正式版创建实例";
  }
});

const createLatestSnapshotButtonText = computed(() => {
  if (createLatestSnapshotErrorOccured.value) {
    return "出现错误";
  } else if (creatingLatestSnapshot.value) {
    return "正在创建...";
  } else {
    return createdLatestSnapshot.value ?? "以最新快照版创建实例";
  }
});
</script>

<style lang="less" scoped>
.import-instances {
  button {
    width: 240px;
    appearance: none;
    background: var(--ctp-lavender);
    color: var(--ctp-text-inverse);
    border: none;
    height: 36px;
    font-size: 12px;
    border-radius: 8px;
    &:hover {
      background-image:
        linear-gradient(#ffffff2f, #ffffff2f),
        linear-gradient(var(--ctp-lavender), var(--ctp-lavender));
    }
    &:active {
      background-image:
        linear-gradient(#ffffff6f, #ffffff6f),
        linear-gradient(var(--ctp-lavender), var(--ctp-lavender));
    }
  }

  button.create-latest-instance {
    &.creating,
    &.error {
      opacity: 0.7;
      pointer-events: none;
    }
    &.error {
      background: var(--ctp-red);
    }
  }
}
</style>
