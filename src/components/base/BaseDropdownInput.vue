<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    class="dropdown-input"
    :style="`width: ${width};`"
    tabindex="0"
    @focusin="onFocusIn"
    @blur="onBlur">
    <div class="input-wrapper" :class="{ focused: opened }">
      <input
        ref="inputRef"
        :type="numberOnly ? 'number' : 'text'"
        :title="name"
        :placeholder="placeholder"
        required
        v-model="inputBoxValue"
        :style="error ? 'outline: rgb(127,0,0)' : ''"
        :disabled="disabled"
        @focus="onFocusIn"
        @keydown="onKeydown" />
      <div class="arrow" :class="{ opened }" @click.stop="toggleDropdown">
        <AppIcon name="chevron-down" :size="14" />
      </div>
    </div>
    <Transition>
      <ul v-if="opened" class="options" :style="{ top: 'calc(100% + 4px)' }">
        <li
          class="dropdown-option"
          v-for="(name, index) in displayName"
          :key="index"
          :class="{ selected: selected === index, highlight: highlight === index }"
          @mousedown.prevent="selectOption(index)"
          @mouseenter="highlight = index">
          {{ name }}
        </li>
      </ul>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "../AppIcon.vue";
import { ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    name?: string;
    placeholder?: string;
    error?: boolean;
    width?: string;
    numberOnly?: boolean;
    disabled?: boolean;
    options: string[];
    displayName: string[];
  }>(),
  {
    width: "400px",
    numberOnly: false,
    disabled: false,
  },
);

const model = defineModel<string>();
const inputBoxValue = ref(model.value ?? "");
const opened = ref(false);
const selected = ref(-1);
const highlight = ref(0);
const inputRef = ref<HTMLInputElement | null>(null);

watch(model, (newValue) => {
  if (newValue !== undefined) {
    inputBoxValue.value = newValue;
    selected.value = props.options.indexOf(newValue);
  }
});

watch(
  () => props.options,
  () => {
    selected.value = props.options.indexOf(model.value ?? "");
  },
  { immediate: true },
);

function onFocusIn() {
  opened.value = true;
  highlight.value = Math.max(selected.value, 0);
}

function onBlur() {
  setTimeout(() => {
    opened.value = false;
    highlight.value = 0;
  }, 150);
}

function toggleDropdown() {
  if (opened.value) {
    opened.value = false;
  } else {
    inputRef.value?.focus();
  }
}

function onKeydown(e: KeyboardEvent) {
  if (!opened.value) return;

  if (e.key === "ArrowDown") {
    e.preventDefault();
    highlight.value = Math.min(highlight.value + 1, props.displayName.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    highlight.value = Math.max(highlight.value - 1, 0);
  } else if (e.key === "Enter") {
    e.preventDefault();
    selectOption(highlight.value);
  } else if (e.key === "Escape") {
    opened.value = false;
    inputRef.value?.blur();
  }
}

function selectOption(index: number) {
  selected.value = index;
  model.value = props.options[index];
  inputBoxValue.value = props.options[index];
  opened.value = false;
  inputRef.value?.blur();
}
</script>

<style lang="less" scoped>
.dropdown-input {
  position: relative;
  height: 30px;
  flex-shrink: 0;
  font-size: 12.5px;
  outline: none;
}

.input-wrapper {
  display: flex;
  align-items: center;
  height: 100%;
  border-radius: var(--controllers-border-radius);
  background: var(--controllers-background);
  border: var(--controllers-border);
  transition: all 0.1s ease;

  &:hover {
    background: var(--controllers-background-hover);
  }

  &.focused {
    outline: var(--controllers-outline-focus);
    background-color: var(--controllers-background-focus);
  }

  input {
    border: none;
    background-color: #00000000;
    padding: 2px 8px;
    height: 100%;
    flex: 1;
    margin: 0;
    font-size: 12.5px;
  }

  input::placeholder {
    color: rgba(var(--default-text-color), 0.8);
  }
}

.arrow {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 100%;
  cursor: pointer;
  transition: transform 0.2s ease;
  flex-shrink: 0;

  &.opened {
    transform: rotate(180deg);
  }
}

.options {
  width: 100%;
  margin: 0;
  padding: 8px;
  list-style: none;
  border-radius: var(--dialog-border-radius);
  border: var(--controllers-border);
  background: var(--dialog-background);
  box-shadow: 0px 0px 10px #4500611d;
  position: absolute;
  left: 0;
  z-index: 100000;
  max-height: 200px;
  overflow-y: auto;
}

.dropdown-option {
  height: 26px;
  padding: 0 8px;
  display: flex;
  align-items: center;
  margin: 2px 0;
  border-radius: var(--controllers-border-radius);
  font-size: 12px;
  transition: all 30ms ease;
  cursor: pointer;

  &:hover {
    background: #ffffff1f;
  }

  &:active {
    background: #ffffff15;
  }

  &.selected {
    background: #ffffff17;
  }

  &.highlight {
    background: #ffffff1f;
  }
}
</style>
