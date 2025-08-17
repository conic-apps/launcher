<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="input-box" :style="`width: ${width};`">
    <input
      @blur="updateModel"
      :type="numberOnly ? 'number' : 'text'"
      :title="name"
      :placeholder="placeholder"
      required
      v-model="inputBoxValue"
      :style="error ? 'outline: rgb(127,0,0)' : ''"
      :disabled="disabled" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    name?: string;
    placeholder?: string;
    type?: string;
    error?: boolean;
    width?: string;
    numberOnly?: boolean;
    disabled?: boolean;
    lazyUpdateModel?: boolean;
    value?: string;
    nonEmpty?: boolean;
  }>(),
  {
    type: "text",
    width: "400px",
    numberOnly: false,
    disabled: false,
  },
);

const model = defineModel();
const inputBoxValue = ref(model.value);
const emits = defineEmits(["updated"]);

if (!props.lazyUpdateModel) {
  watch(inputBoxValue, (newValue) => {
    model.value = newValue;
  });
}

watch(model, (newValue) => {
  inputBoxValue.value = newValue;
});

if (props.value) {
  watch(
    props,
    (newValue) => {
      inputBoxValue.value = newValue;
    },
    {
      immediate: true,
    },
  );
}
function updateModel() {
  if (props.lazyUpdateModel) {
    model.value = inputBoxValue.value;
  }
}

watch(model, () => {
  emits("updated");
});
</script>

<style lang="less" scoped>
.input-box {
  border-radius: var(--controllers-border-radius);
  overflow: hidden;
  height: 30px;
  flex-shrink: 0;
  padding: 0;
  font-size: 15px;
  transition: all 0.1s ease;
  background: var(--controllers-background);
  border: var(--controllers-border);
}

.input-box input {
  border: none;
  background-color: #00000000;
  padding: 0;
  height: 100%;
  width: 100%;
  margin: 0;
  padding: 2px 8px;
  text-align: inherit;
}

.input-box input::placeholder {
  color: rgba(var(--default-text-color), 0.8);
}

.input-box:hover {
  background: var(--controllers-background-hover);
}

.input-box:focus-within {
  outline: var(--controllers-outline-focus);
  background-color: var(--controllers-background-focus);
}
</style>
