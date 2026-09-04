<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="language-setting">
    <p class="wizard-title" ref="title">{{ t("setup.language.title") }}</p>
    <p class="wizard-message" ref="desc1">{{ t("setup.language.desc") }}</p>
    <p class="wizard-message" ref="desc2">
      {{ t("setup.language.desc2") }}
    </p>
    <p class="wizard-message" ref="desc3">
      {{ t("setup.language.desc3") }}
    </p>
    <div class="language-setting-container">
      <button
        class="language-item"
        v-for="(displayName, option) in supportedLanguages"
        ref="language-setting-buttons"
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
import gsap from "gsap";
import { useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

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

const elements = {
  title: useTemplateRef("title"),
  description1: useTemplateRef("desc1"),
  description2: useTemplateRef("desc2"),
  description3: useTemplateRef("desc3"),
  languageSettingButtons: useTemplateRef("language-setting-buttons"),
};

const playIntro = () => {
  return gsap
    .timeline()
    .from(
      elements.title.value,
      {
        opacity: 0,
        scale: 0.8,
        duration: 0.33,
      },
      "<0.03",
    )
    .from(
      elements.description1.value,
      {
        opacity: 0,
        scale: 0.8,
        duration: 0.33,
      },
      "<0.03",
    )
    .from(
      elements.description2.value,
      {
        opacity: 0,
        scale: 0.8,
        duration: 0.33,
      },
      "<0.03",
    )
    .from(
      elements.description3.value,
      {
        opacity: 0,
        scale: 0.8,
        duration: 0.33,
      },
      "<0.03",
    )
    .from(
      elements.languageSettingButtons.value,
      {
        opacity: 0,
        scale: 0.8,
        duration: 0.33,
        stagger: 0.03,
      },
      "<0.03",
    );
};

defineExpose({ playIntro });
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
