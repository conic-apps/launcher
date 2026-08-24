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
const offlineAvatarLoading = ref(false);

watch(
  () => props.uuid,
  async (uuid) => {
    if (props.skin) return;
    offlineAvatarLoading.value = true;
    try {
      const defaultSkin = getDefaultSkin(uuid);
      const skinUrl = defaultSkins[
        `/src/assets/images/skins/${defaultSkin.modelType}/${defaultSkin.textureName}.webp`
      ] as string;
      fallbackAvatar.value = await getAvatarFromUrl(skinUrl, props.size);
      offlineAvatarLoading.value = false;
    } catch {}
  },
  {
    immediate: true,
  },
);

const avatar = ref<string>("");

let currentTask = 0;

const avatarLoading = ref(false);

watch(
  () => props.skin,
  async (skin) => {
    if (!skin) {
      avatar.value = "";
      return;
    }
    const task = ++currentTask;
    avatarLoading.value = true;
    try {
      const result = await getAvatarFromUrl(skin, props.size);

      if (task === currentTask) {
        avatar.value = result;
      }
    } finally {
      if (task === currentTask) {
        avatarLoading.value = false;
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

watch([offlineAvatarLoading, avatarLoading], ([offlineAvatarLoading, avatarLoading]) => {
  if (!offlineAvatarLoading && !avatarLoading) {
    resolveReady();
  }
});

defineExpose({ ready });
</script>

<template>
  <div class="avatar-image">
    <img
      v-if="avatar"
      :src="avatar"
      class="avatar-image"
      :style="{ width: `${size}px`, height: `${size}px` }" />
    <img
      v-else-if="fallbackAvatar"
      :src="fallbackAvatar"
      class="avatar-image"
      :style="{ width: `${size}px`, height: `${size}px` }" />
    <div v-else class="avatar-image placeholder" />
  </div>
</template>

<style scoped>
.avatar-image {
  image-rendering: pixelated;
}

.placeholder {
  background: var(--ctp-surface0);
}
</style>
