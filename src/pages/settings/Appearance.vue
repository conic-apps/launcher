<template>
  <keep-alive>
    <div>
      <setting-group title="默认主题">
        <div class="color-style">
          <div
            :class="{ dark: true, selected: config.appearance.theme == 'dark' }"
            @click="chooseDark">
            <p>深色</p>
          </div>
          <div
            :class="{ light: true, selected: config.appearance.theme == 'light' }"
            @click="chooseLight">
            <p>浅色</p>
          </div>
        </div>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import { watch } from "vue";
import $ from "jquery";
import { reloadTheme } from "@/theme";
const config = useConfigStore();

watch(
  config,
  () => {
    if (config.accessibility.disable_animations) {
      $("body").addClass("no-animations");
    } else {
      $("body").removeClass("no-animations");
    }
  },
  {},
);

const chooseDark = () => {
  config.appearance.theme = "dark";
  reloadTheme(config);
};
const chooseLight = () => {
  config.appearance.theme = "light";
  reloadTheme(config);
};
</script>

<style lang="less" scoped>
.color-style {
  width: 100%;
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--setting-item-background);

  > div {
    width: 160px;
    height: 120px;
    margin: 0 40px 20px 40px;
    background-position: center;
    background-size: 114%;
    border-radius: 12px;
    transition: all 100ms ease;

    p {
      width: 100%;
      text-align: center;
      margin-top: calc(100% - 26px);
    }
  }

  .dark {
    background-image: url("@/assets/images/dark.png");
  }

  .light {
    background-image: url("@/assets/images/light.png");
  }

  .selected {
    outline: 4px solid rgb(24, 170, 255);
  }
}
</style>
