<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    name: string;
    size?: number | string;
    stroke?: string;
    fill?: string;
  }>(),
  {
    size: 20,
    stroke: "currentColor",
    fill: "currentColor",
  },
);

const icons = import.meta.glob("@/assets/icons/*.svg", {
  eager: true,
  import: "default",
  query: "?component",
});

const iconMap = Object.fromEntries(
  Object.entries(icons).map(([path, component]) => [
    path.split("/").pop()?.replace(".svg", ""),
    component,
  ]),
);

const IconComponent = computed(() => iconMap[props.name]);
</script>

<template>
  <component
    :is="IconComponent"
    :width="size"
    :height="size"
    :stroke="stroke"
    :fill="fill"
    class="inline-block"
    :style="{ stroke, fill }" />
</template>
