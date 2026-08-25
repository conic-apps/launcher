<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="search-bar" :style="style">
    <AppIcon name="search"></AppIcon>
    <input
      type="text"
      v-model="value"
      @input="$emit('search', value)"
      :placeholder="placeholderText"
      autocapitalize="off"
      autocomplete="off"
      autocorrect="off"
      :spellcheck="false" />
  </div>
</template>

<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
const { t } = useI18n();
const value = ref("");
const props = withDefaults(
  defineProps<{
    width?: string;
    height?: string;
    placeholder?: string;
  }>(),
  {
    width: "",
    height: "",
    placeholder: "",
  },
);
const style = `width: ${props.width}; height: ${props.height};`;
const placeholderText = computed(() => props.placeholder || t("app.search"));
</script>

<style lang="less" scoped>
.search-bar {
  display: flex;
  align-items: center;
  background: var(--controllers-background);
  border: var(--controllers-border);
  border-radius: var(--controllers-border-radius);
  padding: 0px 10px;
  width: 472px;
  height: 26px;
  position: relative;
  margin: 0 auto;
  top: 0px;
  bottom: 0px;
  left: 0px;
  right: 0px;
  z-index: 10;
  transition: all 0.3s ease;
}

.search-bar i {
  font-family: "fa-pro";
  font-style: normal;
  font-size: 10px;
  margin-right: 6px;
  color: rgba(var(--default-text-color), 0.8);
  transform: scale(0.95);
}

.search-bar input {
  border: none;
  background: none;
  outline: none;
  font-size: 13px;
  color: rgba(var(--default-text-color), 0.8);
  width: 100%;
  margin-left: 6px;
}

.search-bar input::placeholder {
  color: rgba(var(--default-text-color), 0.8);
  display: none;
}
</style>
