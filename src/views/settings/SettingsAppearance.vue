<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
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
    <SettingGroup title="背景图像">
      <SettingItem
        title="自定义背景图像"
        description="选择一张图片作为启动器背景，优先级低于实例背景"
        icon="image"
        :navigable="!config.appearance.background_image"
        @click="pickBackgroundImage">
        <AppIcon
          name="chevron-forward"
          style="margin-right: 4px"
          v-if="!config.appearance.background_image"></AppIcon>
        <BaseButton color="var(--ctp-red)" v-else @click.stop="removeBackgroundImageSetting"
          >移除图像</BaseButton
        >
      </SettingItem>
      <SettingItem
        title="立体背景摄像机移动"
        description="关闭后摄像机停止向前移动，背景渲染完成后不再更新，以完全关闭背景开销"
        icon-fill="none">
        <BaseSwitch v-model="config.appearance.background_camera_move"></BaseSwitch>
      </SettingItem>
      <SettingItem title="背景图片视差" description="关闭后背景不再随鼠标移动" icon-fill="none">
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
import AppIcon from "@/components/AppIcon.vue";
import { useConfigStore } from "@/store/config";
import { ref, watch } from "vue";
import { reloadPalette } from "@/theme";
import { Palette, setBackgroundImage, removeBackgroundImage } from "@conic/config";
import { open } from "@tauri-apps/plugin-dialog";
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
