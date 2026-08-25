<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="language-setting">
    <p class="wizard-title">欢迎</p>
    <p class="wizard-message">欢迎来到首次启动设置指南页！</p>
    <p class="wizard-message">
      Conic Launcher 是可高度自定义的启动器，但那些复杂的设置可能会让你不知所措。
    </p>
    <p class="wizard-message">
      这个指南将快速带你完成一些重要的设置项，以便待会儿你能够快速上手并启动你的游戏。
    </p>
    <div class="language-setting-container">
      <button
        class="language-item"
        v-for="(displayName, option) in supportedLanguages"
        :key="option"
        :class="{ selected: configStore.language === option }"
        @click="configStore.language = option">
        {{ displayName }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useConfigStore } from "@/store/config";

const configStore = useConfigStore();

const supportedLanguages = {
  en_us: "English",
  zh_cn: "简体中文",
  zh_tw: "繁體中文",
  ja_jp: "日本語",
  ko_kr: "한국어",
  de_de: "Deutsch",
  fr_fr: "Français",
  es_es: "Español",
  pt_br: "Português (Brasil)",
  ru_ru: "Русский",
  tr_tr: "Türkçe",
  pl_pl: "Polski",
};
</script>

<style lang="less" scoped>
.language-setting-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
  gap: 8px;
  margin-top: 16px;
  button.language-item {
    height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: none;
    border-radius: 8px;
    transition: background 100ms ease;
    &:hover {
      background: var(--ctp-surface0);
      transition: none;
    }
    &:active {
      background: var(--ctp-surface1);
      transition: background 200ms ease;
    }
    &.selected {
      background: var(--ctp-surface1);
      color: var(--ctp-mauve);
      transition: background 200ms ease;
    }
  }
}
</style>
