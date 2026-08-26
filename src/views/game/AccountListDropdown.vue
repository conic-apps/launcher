<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="account-list-dropdown" ref="dropdownRef">
    <slot name="trigger" :opened="opened" :toggle="toggle"></slot>
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
        <ul class="dropdown" v-if="opened">
          <slot name="content" :opened="opened" :toggle="toggle"></slot>
        </ul>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useDismissOnOutsidePointerDown, useDropdownTransition } from "./useDropdownTransition";

const props = withDefaults(
  defineProps<{
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
} = useDropdownTransition(opened, props);

function toggle() {
  opened.value = !opened.value;
}
</script>

<style lang="less" scoped>
.account-list-dropdown {
  position: relative;
  margin-left: -16px;
  margin-bottom: 32px;
  background: var(--ctp-surface0);
  height: 36px;
  border-radius: 8px;
  font-size: 14px;
  display: flex;
  align-items: center;
  padding: 0 0 0 32px;
  box-shadow: 0 0 10px 0px rgba(0, 0, 0, 0.2);

  :deep(.account-switch) {
    appearance: none;
    background: var(--ctp-surface1);
    margin-left: 16px;
    height: 100%;
    width: 36px;
    border: none;
    border-radius: 0 8px 8px 0;

    &:hover {
      background: var(--ctp-surface2);
    }

    &:active {
      background: var(--ctp-overlay0);
    }
  }

  .dropdown {
    position: absolute;
    right: 0;
    bottom: calc(100% + 4px);
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
      gap: 8px;
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

      .player-name {
        overflow: hidden;
        text-overflow: ellipsis;
      }
    }
  }
}
</style>
