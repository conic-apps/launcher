<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<script setup lang="ts">
import { getAvatarFromUrl, getDefaultSkin } from "@conic/account";
import { ref, watch } from "vue";

const props = defineProps<{
  skin?: string;
  uuid: string;
  size: number;
}>();

const defaultSkins = import.meta.glob("@/assets/images/skins/**/*.webp", {
  eager: true,
  import: "default",
});
const fallbackAvatar = ref("");

watch(
  () => props.uuid,
  async (uuid) => {
    const defaultSkin = getDefaultSkin(uuid);
    const skinUrl = defaultSkins[
      `/src/assets/images/skins/${defaultSkin.modelType}/${defaultSkin.textureName}.webp`
    ] as string;
    fallbackAvatar.value = await getAvatarFromUrl(skinUrl, props.size);
  },
  {
    immediate: true,
  },
);

const avatar = ref<string>("");
const loading = ref(false);

let currentTask = 0;

watch(
  () => props.skin,
  async (skin) => {
    if (!skin) {
      avatar.value = "";
      return;
    }
    const task = ++currentTask;
    loading.value = true;
    try {
      const result = await getAvatarFromUrl(skin, props.size);

      if (task === currentTask) {
        avatar.value = result;
      }
    } finally {
      if (task === currentTask) {
        loading.value = false;
      }
    }
  },
  {
    immediate: true,
  },
);

let resolveReady: () => void;

const ready = new Promise<void>((resolve) => {
  resolveReady = resolve;
});

watch(loading, (loading) => {
  if (!loading) {
    resolveReady();
  }
});

defineExpose({ ready });
</script>

<template>
  <img
    v-if="avatar"
    :src="avatar"
    class="avatar"
    :style="{ width: `${size}px`, height: `${size}px` }" />
  <img
    v-else-if="fallbackAvatar"
    :src="fallbackAvatar"
    class="avatar"
    :style="{ width: `${size}px`, height: `${size}px` }" />
  <div v-else class="avatar placeholder" />
</template>

<style scoped>
.avatar {
  image-rendering: pixelated;
}

.placeholder {
  background: var(--ctp-surface0);
}
</style>
