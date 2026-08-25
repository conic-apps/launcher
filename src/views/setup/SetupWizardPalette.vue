<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="choose-palette">
    <p class="wizard-title">选择配色方案</p>
    <p class="wizard-message">Conic Launcher 支持多种配色方案，你可以按自己的喜好选择一个。</p>
    <div class="palette-setting-container">
      <SettingGroup style="width: 100%; margin-top: 16px">
        <SettingItem
          title="跟随系统深色设置"
          description="如果系统设置中设置为浅色，则使用 Latte ，否则使用 Mocha"
          icon="moon"
          icon-fill="none">
          <BaseSwitch v-model="configStore.appearance.palette_follow_system"></BaseSwitch>
        </SettingItem>
        <div
          class="color-style"
          :class="{
            'color-style-disabled': configStore.appearance.palette_follow_system,
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
    </div>
  </div>
</template>

<script setup lang="ts">
import BaseSwitch from "@/components/BaseSwitch.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import SettingItem from "@/components/SettingItem.vue";
import { useConfigStore } from "@/store/config";
import { reloadPalette } from "@/theme";
import { Palette } from "@conic/config";
import { ref, watch } from "vue";

const configStore = useConfigStore();

const currentTheme = ref<Palette>(configStore.appearance.palette);

const choosePalette = (palette: Palette) => {
  configStore.appearance.palette = palette;
  currentTheme.value = palette;
  reloadPalette(
    {
      palette,
      paletteFollowSystem: configStore.appearance.palette_follow_system,
    },
    configStore.accessibility.high_contrast_mode,
  );
};

watch(
  () => configStore.appearance.palette_follow_system,
  (paletteFollowSystem) => {
    reloadPalette(
      {
        palette: configStore.appearance.palette,
        paletteFollowSystem,
      },
      configStore.accessibility.high_contrast_mode,
    );
    if (paletteFollowSystem) {
      if (matchMedia.matches) {
        currentTheme.value = Palette.Mocha;
      } else {
        currentTheme.value = Palette.Latte;
      }
    } else {
      currentTheme.value = configStore.appearance.palette;
    }
  },
);
const matchMedia = window.matchMedia("(prefers-color-scheme: dark)");
matchMedia.addEventListener("change", (event) => {
  if (configStore.appearance.palette_follow_system) {
    if (event.matches) {
      currentTheme.value = Palette.Mocha;
    } else {
      currentTheme.value = Palette.Latte;
    }
  } else {
    currentTheme.value = configStore.appearance.palette;
  }
  reloadPalette(
    {
      palette: configStore.appearance.palette,
      paletteFollowSystem: configStore.appearance.palette_follow_system,
    },
    configStore.accessibility.high_contrast_mode,
  );
});
if (configStore.appearance.palette_follow_system) {
  if (matchMedia.matches) {
    currentTheme.value = Palette.Mocha;
  } else {
    currentTheme.value = Palette.Latte;
  }
}
</script>

<style lang="less" scoped>
.palette-setting-container {
  width: 100%;
}

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
    margin: -24px 12px 0px 12px;
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
    background-image: url("@/assets/images/catppuccin-latte.webp");
  }

  .frappe {
    background-image: url("@/assets/images/catppuccin-frappe.webp");
  }

  .macchiato {
    background-image: url("@/assets/images/catppuccin-macchiato.webp");
  }

  .mocha {
    background-image: url("@/assets/images/catppuccin-mocha.webp");
  }

  .selected {
    outline: 4px solid var(--ctp-blue);
  }
}

.color-style-disabled {
  pointer-events: none;
  * {
    opacity: 0.6;
  }
}
</style>
