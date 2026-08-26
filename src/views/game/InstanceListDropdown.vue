<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list-dropdown" ref="dropdownRef">
    <div class="head" @click="opened = !opened">
      <div class="label">{{ label }}</div>
      <div class="selected" :style="selectedWidth !== undefined ? `width: ${selectedWidth}px` : ''">
        <span class="text">{{ selected }}</span>
        <span class="chevron" ref="chevronRef">
          <AppIcon name="chevron-down" :size="14"></AppIcon>
        </span>
      </div>
    </div>
    <div>
      <Transition
        :css="false"
        @before-enter="onBeforeEnter"
        @enter="onEnter"
        @after-enter="onAfterEnter"
        @enter-cancelled="onEnterCancelled"
        @before-leave="onBeforeLeave"
        @leave="onLeave"
        @after-leave="onAfterLeave"
        @leave-cancelled="onLeaveCancelled">
        <ul class="dropdown" v-if="opened" @click="opened = false">
          <slot></slot>
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { ref } from "vue";
import {
  flipDropdownChevron,
  useDismissOnOutsidePointerDown,
  useDropdownTransition,
} from "./useDropdownTransition";

const props = withDefaults(
  defineProps<{
    label: string;
    selected: string;
    selectedWidth?: number;
    expandDuration?: number;
    collapseDuration?: number;
    expandEasing?: string;
    collapseEasing?: string;
  }>(),
  {
    expandDuration: 200,
    collapseDuration: 200,
    expandEasing: "ease",
    collapseEasing: "ease",
  },
);

const opened = ref(false);

const dropdownRef = ref<HTMLElement | null>(null);
const chevronRef = ref<HTMLElement | null>(null);

useDismissOnOutsidePointerDown(dropdownRef, opened);

const {
  onBeforeEnter,
  onEnter,
  onAfterEnter,
  onEnterCancelled,
  onBeforeLeave,
  onLeave,
  onAfterLeave,
  onLeaveCancelled,
} = useDropdownTransition(opened, {
  expandDuration: props.expandDuration,
  collapseDuration: props.collapseDuration,
  expandEasing: props.expandEasing,
  collapseEasing: props.collapseEasing,
  onChange: (value) => {
    flipDropdownChevron(chevronRef.value, value ? 180 : 0);
  },
});
</script>

<style lang="less" scoped>
.instances-list-dropdown {
  position: relative;
  width: 100%;

  .head {
    display: flex;
    width: 100%;
    align-items: center;
    font-size: 13px;
    background: rgba(var(--ctp-surface0-rgb), 1);
    border-radius: 4px;

    .label {
      background: rgba(var(--ctp-surface1-rgb), 1);
      padding: 6px 12px;
      border-radius: 4px;
      flex-shrink: 0;
    }

    .selected {
      padding: 0 12px;
      flex: 1;
      height: 100%;
      display: flex;
      align-items: center;
      overflow: hidden;

      .text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .chevron {
        margin-left: auto;
        display: inline-flex;
        align-items: center;
      }
    }
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 100%;
    padding: 8px 10px;
    border-radius: var(--dialog-border-radius);
    border: var(--controllers-border);
    background: var(--ctp-base);
    box-shadow: 0px 0px 10px #4500611d;
    z-index: 100000;
    list-style: none;

    :deep(.dropdown-option) {
      height: 26px;
      padding: 0 8px;
      display: flex;
      align-items: center;
      margin: 4px 0;
      border-radius: var(--controllers-border-radius);
      font-size: 12px;
      list-style: none;
      white-space: nowrap;
      transition: all 30ms ease;

      &:hover {
        background: #ffffff1f;
      }

      &:active {
        background: #ffffff15;
      }

      &.selected {
        background: #ffffff17;
      }
    }
  }
}
</style>
