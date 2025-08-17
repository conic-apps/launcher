<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <InfoBox :click-able="true" @click="openUrl('https://catppuccin.com')">
      <h2
        :style="{
          'margin-bottom': '8px',
          'background-image': `linear-gradient(120deg, var(--ctp-${config.appearance.palette.toLowerCase()}-peach), var(--ctp-${config.appearance.palette.toLowerCase()}-mauve))`,
          'background-clip': 'text',
          color: '#00000000',
        }">
        默认调色板由 Cappuccin 驱动
      </h2>
      Catppuccin 是一款由社区驱动的柔和色调主题，旨在成为低对比度和高对比度主题之间的平衡点。它包含
      4 种舒缓的暖色调，每种都有 26 种赏心悦目的颜色，点击此处以查看详细信息
    </InfoBox>
    <setting-group title="调色板">
      <setting-item
        title="跟随系统深色设置"
        description="如果系统设置中设置为浅色，则使用 Latte 调色板，否则使用 Mocha 调色板"
        icon="moon"
        icon-fill="none">
        <toggle-switch v-model="config.appearance.palette_follow_system"></toggle-switch>
      </setting-item>
      <div
        :class="{
          'color-style': true,
          'color-style-unavailable': config.appearance.palette_follow_system,
        }">
        <div
          :class="{ latte: true, selected: currentTheme == Palette.Latte }"
          @click="choosePalette(Palette.Latte)">
          <p>Latte</p>
        </div>
        <div
          :class="{ frappe: true, selected: currentTheme == Palette.Frappe }"
          @click="choosePalette(Palette.Frappe)">
          <p>Frappé</p>
        </div>
        <div
          :class="{ macchiato: true, selected: currentTheme == Palette.Macchiato }"
          @click="choosePalette(Palette.Macchiato)">
          <p>Macchiato</p>
        </div>
        <div
          :class="{ mocha: true, selected: currentTheme == Palette.Mocha }"
          @click="choosePalette(Palette.Mocha)">
          <p>Mocha</p>
        </div>
      </div>
    </setting-group>
  </div>
</template>

<script setup lang="ts">
//TODO: 主题色，自定义样式表
import SettingGroup from "@/components/SettingGroup.vue";
import InfoBox from "@/components/InfoBox.vue";
import SettingItem from "@/components/SettingItem.vue";
import ToggleSwitch from "@/components/ToggleSwitch.vue";
import { useConfigStore } from "@/store/config";
import { ref, watch } from "vue";
import { reloadPalette } from "@/theme";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Palette } from "@conic/config";
const config = useConfigStore();

const currentTheme = ref<Palette>(config.appearance.palette);

const choosePalette = (palette: Palette) => {
  config.appearance.palette = palette;
  currentTheme.value = palette;
  reloadPalette(
    {
      palette,
      paletteFollowSystem: config.appearance.palette_follow_system,
    },
    config.accessibility.high_contrast_mode,
  );
};

watch(
  () => config.appearance.palette_follow_system,
  (paletteFollowSystem) => {
    reloadPalette(
      {
        palette: config.appearance.palette,
        paletteFollowSystem,
      },
      config.accessibility.high_contrast_mode,
    );
    if (paletteFollowSystem) {
      if (matchMedia.matches) {
        currentTheme.value = Palette.Mocha;
      } else {
        currentTheme.value = Palette.Latte;
      }
    } else {
      currentTheme.value = config.appearance.palette;
    }
  },
);
const matchMedia = window.matchMedia("(prefers-color-scheme: dark)");
matchMedia.addEventListener("change", (event) => {
  if (config.appearance.palette_follow_system) {
    if (event.matches) {
      currentTheme.value = Palette.Mocha;
    } else {
      currentTheme.value = Palette.Latte;
    }
  } else {
    currentTheme.value = config.appearance.palette;
  }
  reloadPalette(
    {
      palette: config.appearance.palette,
      paletteFollowSystem: config.appearance.palette_follow_system,
    },
    config.accessibility.high_contrast_mode,
  );
});
if (config.appearance.palette_follow_system) {
  if (matchMedia.matches) {
    currentTheme.value = Palette.Mocha;
  } else {
    currentTheme.value = Palette.Latte;
  }
}
</script>

<style lang="less" scoped>
.color-style {
  width: 100%;
  height: 160px;
  display: flex;
  padding: 0 24px 0 24px;
  align-items: center;
  justify-content: center;
  background: var(--setting-item-background);

  > div {
    width: 120px;
    height: 74px;
    margin: -24px 20px 0px 20px;
    background-position: center;
    background-size: 100%;
    border-radius: 6px;
    transition: all 100ms ease;

    p {
      width: 100%;
      text-align: center;
      margin-top: calc(100% - 30px);
    }
  }

  .latte {
    background-image: url("../../assets/images/catppuccin-latte.webp");
  }

  .frappe {
    background-image: url("../../assets/images/catppuccin-frappe.webp");
  }

  .macchiato {
    background-image: url("../../assets/images/catppuccin-macchiato.webp");
  }

  .mocha {
    background-image: url("../../assets/images/catppuccin-mocha.webp");
  }

  .selected {
    outline: 4px solid rgb(24, 170, 255);
  }
}
.color-style-unavailable {
  pointer-events: none;
  * {
    opacity: 0.6;
  }
}
</style>
