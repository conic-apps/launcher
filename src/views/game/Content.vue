<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-content-container">
    <component :is="components[currentComponentName]"></component>
  </div>
</template>

<script setup lang="ts">
import { markRaw } from "vue";
import ContentMods from "./ContentMods.vue";
import ContentResourcepacks from "./ContentResourcepacks.vue";
import ContentScreenshots from "./ContentScreenshots.vue";
import ContentSaves from "./ContentSaves.vue";
import AppIcon from "@/components/AppIcon.vue";
import { useContentComponent } from "./useContent";
import BaseSelect from "@/components/BaseSelect.vue";

const props = withDefaults(
  defineProps<{
    currentComponent?: "mods" | "worlds" | "resourcepacks" | "screenshots";
  }>(),
  { currentComponent: "worlds" },
);

const components = {
  saves: markRaw(ContentSaves),
  mods: markRaw(ContentMods),
  resourcepacks: markRaw(ContentResourcepacks),
  screenshots: markRaw(ContentScreenshots),
};
export type ComponentName = keyof typeof components;

const currentComponentName = useContentComponent();
</script>

<style lang="less" scoped>
.game-content-container {
  background: var(--ctp-base);
  width: calc(100% - 150px);
  height: 100%;
  display: flex;
  flex-direction: column;
}
</style>
