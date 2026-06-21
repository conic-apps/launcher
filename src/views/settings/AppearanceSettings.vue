<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <InfoBox :click-able="true" @click="openUrl('https://catppuccin.com')">
      <p
        :style="{
          marginBottom: '8px',
          backgroundImage: `linear-gradient(120deg, var(--ctp-${config.appearance.palette.toLowerCase()}-peach), var(--ctp-${config.appearance.palette.toLowerCase()}-mauve))`,
          backgroundClip: 'text',
          color: '#00000000',
          fontSize: '18px',
          fontWeight: 'bold',
        }">
        Default pastel theme is Catppuccin
      </p>
      Catppuccin is a community-driven color scheme meant for coding, designing, and much more.
      Catppuccin consists of 4 beautiful pastel color palettes, named flavors. You can choose one
      below. Click here for more information.
    </InfoBox>
    <SettingGroup title="调色板">
      <SettingItem
        title="跟随系统深色设置"
        description="如果系统设置中设置为浅色，则使用 Latte ，否则使用 Mocha"
        icon="moon"
        icon-fill="none">
        <BaseSwitch v-model="config.appearance.palette_follow_system"></BaseSwitch>
      </SettingItem>
      <div
        class="color-style"
        :class="{
          'color-style-disabled': config.appearance.palette_follow_system,
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
    </SettingGroup>
    <!-- TODO: Font settings -->
  </div>
</template>

<script setup lang="ts">
//TODO: 主题色，自定义样式表
import SettingGroup from "@/components/SettingGroup.vue";
import InfoBox from "@/components/InfoBox.vue";
import SettingItem from "@/components/SettingItem.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
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
  height: 120px;
  display: flex;
  padding: 0 16px;
  align-items: center;
  justify-content: center;
  background: var(--setting-item-background);

  > div {
    width: 90px;
    height: 60px;
    margin: -24px 10px 0px 10px;
    background-position: center;
    background-size: 100%;
    border-radius: 6px;
    transition: all 100ms ease;

    p {
      width: 100%;
      text-align: center;
      font-size: 12.3px;
      margin-top: calc(100% - 20px);
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
.color-style-disabled {
  pointer-events: none;
  * {
    opacity: 0.6;
  }
}
</style>
