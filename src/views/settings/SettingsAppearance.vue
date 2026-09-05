<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="t('settings.appearance.palette')">
      <SettingItem
        :title="t('settings.appearance.followSystem')"
        :description="t('settings.appearance.followSystemDesc')"
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
    <SettingGroup :title="t('settings.appearance.backgroundImage')">
      <SettingItem
        :title="t('settings.appearance.customBackground')"
        :description="t('settings.appearance.customBackgroundDesc')"
        icon="image"
        :navigable="!config.appearance.background_image"
        @click="pickBackgroundImage">
        <AppIcon
          name="chevron-forward"
          style="margin-right: 4px"
          v-if="!config.appearance.background_image"></AppIcon>
        <BaseButton color="var(--ctp-red)" v-else @click.stop="removeBackgroundImageSetting">{{
          t("settings.appearance.removeImage")
        }}</BaseButton>
      </SettingItem>
      <SettingItem
        :title="t('settings.appearance.backgroundDarkness')"
        :description="`${config.appearance.background_darkness}%`"
        :disabled="!config.appearance.background_image">
        <BaseSliderBar
          :max="100"
          :min="0"
          :step="1"
          :disabled="!config.appearance.background_image"
          v-model="config.appearance.background_darkness"></BaseSliderBar>
      </SettingItem>
      <SettingItem
        :title="t('settings.appearance.parallaxCamera')"
        :description="t('settings.appearance.parallaxCameraDesc')"
        icon-fill="none">
        <BaseSwitch v-model="config.appearance.background_camera_move"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.appearance.parallaxMouse')"
        :description="t('settings.appearance.parallaxMouseDesc')"
        icon-fill="none">
        <BaseSwitch v-model="config.appearance.background_parallax"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingGroup from "@/components/SettingGroup.vue";
import SettingItem from "@/components/SettingItem.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import BaseButton from "@/components/BaseButton.vue";
import BaseSliderBar from "@/components/BaseSliderBar.vue";
import AppIcon from "@/components/AppIcon.vue";
import { useConfigStore } from "@/store/config";
import { ref, watch } from "vue";
import { reloadPalette } from "@/theme";
import { Palette, setBackgroundImage, removeBackgroundImage } from "@conic/config";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
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

async function pickBackgroundImage() {
  const filePath = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg", "ico"],
      },
    ],
  });
  if (filePath) {
    const filename = await setBackgroundImage(filePath);
    config.appearance.background_image = filename;
  }
}

async function removeBackgroundImageSetting() {
  await removeBackgroundImage();
  config.appearance.background_image = undefined;
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
