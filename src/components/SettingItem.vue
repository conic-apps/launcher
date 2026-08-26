<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    class="setting-item"
    :class="{ 'setting-item-navigable': props.navigable, 'setting-item-disabled': props.disabled }">
    <div style="display: flex">
      <div class="icon" v-if="icon || slots.icon">
        <AppIcon v-if="icon" :name="icon" :size="iconSize" :fill="iconFill"></AppIcon>
        <slot name="icon" v-else-if="slots.icon"> </slot>
      </div>
      <div class="text">
        <slot v-if="slots.title" name="title"> </slot>
        <p class="title" v-else-if="props.title">
          {{ title }}
          <button v-if="props.resetable" class="reset-button" @click.stop="$emit('reset')">
            <AppIcon name="refresh" :size="16"></AppIcon>
          </button>
          <Transition name="fade">
            <span
              style="
                transform: scale(0.8);
                height: 100%;
                margin-left: 4px;
                display: flex;
                align-items: center;
              "
              v-if="props.loading">
              <BaseLoading :size="15" :stroke-width="8" :gap="8"></BaseLoading>
            </span>
          </Transition>
        </p>
        <p v-if="description" class="description" v-html="description"></p>
        <slot v-else-if="slots.description" name="description"> </slot>
      </div>
    </div>
    <div style="display: flex; align-items: center">
      <AppIcon
        name="chevron-forward"
        style="margin-right: 4px"
        :fill="props.iconFill"
        v-if="props.navigable"
        :size="17"></AppIcon>
      <slot v-else></slot>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useSlots } from "vue";
import AppIcon from "./AppIcon.vue";
import BaseLoading from "./BaseLoading.vue";

const props = withDefaults(
  defineProps<{
    title?: string;
    description?: string;
    icon?: string;
    iconSize?: number | string;
    iconFill?: string;
    navigable?: boolean;
    disabled?: boolean;
    resetable?: boolean;
    loading?: boolean;
  }>(),
  {
    iconSize: 26,
  },
);

defineEmits(["reset"]);

const slots = useSlots();
</script>

<style lang="less" scoped>
.setting-item {
  padding: 10px 16px 10px 14px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: 0;
  margin-bottom: 1px;
  background: var(--setting-item-background);
  transition: all 50ms ease;

  > * {
    transition: all 100ms ease;
  }

  .icon,
  .text {
    display: flex;
    gap: 2px;
    flex-direction: column;
    justify-content: center;
  }

  .icon {
    align-items: center;
    width: 40px;
    height: inherit;
    margin-right: 8px;
    flex-shrink: 0;
  }

  .text {
    align-items: flex-start;
  }

  .text p.title {
    display: flex;
    align-items: center;
    font-weight: normal;
    font-size: 13px;
  }

  button.reset-button {
    display: inline-flex;
    justify-content: center;
    align-items: center;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    margin-left: 3px;
    appearance: none;
    border: none;
    background: none;
  }

  button.reset-button:hover {
    background: #ffffff1c;
  }

  button.reset-button:active {
    opacity: 0.8;
  }

  .text p.description {
    font-size: 12px;
    color: rgba(var(--default-text-color), 0.849);
    line-height: 1.1;
    opacity: 0.6;
    margin-top: 2px;
  }
}

.setting-item-navigable:hover {
  background: var(--setting-item-background-hover);
}

.setting-item-navigable:active {
  background-color: var(--setting-item-background-active);
}

.setting-item-disabled {
  > * {
    opacity: 0.6;
  }

  pointer-events: none;
}
</style>
