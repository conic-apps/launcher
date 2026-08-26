<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="import-instances">
    <p class="wizard-title">{{ t("setup.importInstances.title") }}</p>
    <p class="wizard-message">{{ t("setup.importInstances.desc") }}</p>
    <div style="width: 100%; display: flex; justify-content: center; margin: 12px 0">
      <button class="import-from-other-launcher">
        {{ t("setup.importInstances.importButton") }}
      </button>
    </div>
    <p class="wizard-message">
      {{ t("setup.importInstances.newPlayerHint") }}
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
import { useI18n } from "vue-i18n";

const { t } = useI18n();

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
      name: t("setup.importInstances.latestRelease"),
      runtime: { minecraft: minecraftVersionManifest.latest.release },
    });
    createdLatestRelease.value = minecraftVersionManifest.latest.release;
  } catch (error) {
    console.error("Failed to create latest release instance", error);
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
      name: t("setup.importInstances.latestSnapshot"),
      runtime: { minecraft: minecraftVersionManifest.latest.snapshot },
    });
    createdLatestSnapshot.value = minecraftVersionManifest.latest.snapshot;
  } catch (error) {
    console.error("Failed to create latest snapshot instance", error);
    createLatestSnapshotErrorOccured.value = true;
  } finally {
    creatingLatestSnapshot.value = false;
  }
}

const createLatestReleaseButtonText = computed(() => {
  if (createLatestReleaseErrorOccured.value) {
    return t("setup.importInstances.error");
  } else if (creatingLatestRelease.value) {
    return t("setup.importInstances.creating");
  } else {
    return createdLatestRelease.value ?? t("setup.importInstances.createRelease");
  }
});

const createLatestSnapshotButtonText = computed(() => {
  if (createLatestSnapshotErrorOccured.value) {
    return t("setup.importInstances.error");
  } else if (creatingLatestSnapshot.value) {
    return t("setup.importInstances.creating");
  } else {
    return createdLatestSnapshot.value ?? t("setup.importInstances.createSnapshot");
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
