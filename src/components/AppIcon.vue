<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";

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

// eslint-disable-next-line
const IconComponent = computed(() =>
  defineAsyncComponent(() => import(`@/assets/icons/${props.name}.svg?component`)),
);
</script>

<template>
  <component
    :is="IconComponent"
    :width="size"
    :height="size"
    :stroke="stroke"
    :fill="fill"
    class="inline-block" />
</template>
