<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="settings-view">
    <div class="column-left">
      <ul class="settings-menu">
        <li
          @click="switchComponent(item.component, index)"
          :class="{ active: activeComponentIndex == index }"
          v-for="(item, index) in components"
          :key="index">
          <AppIcon :name="item.icon" :size="16"></AppIcon><span>{{ $t(item.name) }}</span>
        </li>
      </ul>
    </div>
    <div class="column-right">
      <Transition :name="transitionName" mode="out-in">
        <component :is="currentComponent"></component>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { type Component, markRaw, ref, shallowRef } from "vue";
import GeneralSettings from "./settings/GeneralSettings.vue";
import GameSettings from "./settings/GameSettings.vue";
import JvmSettings from "./settings/JvmSettings.vue";
import AppearanceSettings from "./settings/AppearanceSettings.vue";
import DownloadSettings from "./settings/DownloadSettings.vue";
import AccessibilitySettings from "./settings/AccessibilitySettings.vue";
import AboutSettings from "./settings/AboutSettings.vue";
import AppIcon from "@/components/AppIcon.vue";
import AudioSettings from "./settings/AudioSettings.vue";

const components = ref<{ name: string; icon: string; component: Component }[]>([
  {
    name: "settings.general.sidebar",
    icon: "house",
    component: markRaw(GeneralSettings),
  },
  {
    name: "settings.game.sidebar",
    icon: "gamepad",
    component: markRaw(GameSettings),
  },
  {
    name: "settings.game.jvmSidebar",
    icon: "java",
    component: markRaw(JvmSettings),
  },
  {
    name: "settings.appearance.sidebar",
    icon: "palette",
    component: markRaw(AppearanceSettings),
  },
  {
    name: "音乐与音效",
    icon: "musical-notes",
    component: markRaw(AudioSettings),
  },
  {
    name: "settings.network.sidebar",
    icon: "globe",
    component: markRaw(DownloadSettings),
  },
  {
    name: "settings.accessibility.sidebar",
    icon: "accessibility",
    component: markRaw(AccessibilitySettings),
  },
  {
    name: "settings.about.sidebar",
    icon: "about",
    component: markRaw(AboutSettings),
  },
]);
const currentComponent = shallowRef(components.value[0].component);
const activeComponentIndex = ref(0);
const transitionName = ref("slide-up");
function switchComponent(component: Component, index: number) {
  if (activeComponentIndex.value < index) {
    transitionName.value = "slide-up";
  } else {
    transitionName.value = "slide-down";
  }
  currentComponent.value = component;
  activeComponentIndex.value = index;
}
</script>

<style lang="less" scoped>
.settings-view {
  width: 100%;
  height: 100%;
  display: flex;
  background: rgba(var(--ctp-base-rgb), 0.8);
  .column-left,
  .column-right {
    height: 100%;
  }

  .column-left {
    width: 224px;
    flex-shrink: 0;
    padding: 28px 0px 16px 32px;
  }

  .column-right {
    width: 100%;
    padding: 24px 24px 24px 0;
    padding-left: 16px;
    overflow: auto;
  }

  .settings-menu {
    height: calc(100% - 40px);
    span {
      margin-left: 8px;
    }
    li {
      height: 30px;
      width: 100%;
      display: flex;
      align-items: center;
      padding-left: 10px;
      font-size: 13px;
      border-radius: 8px;
      margin-bottom: 4px;
    }
    li:hover {
      background: rgba(var(--ctp-surface0-rgb), 0.8);
    }
    li.active {
      background: rgba(var(--ctp-surface1-rgb), 0.8);
    }
    li::before {
      content: "";
      width: 3px;
      height: 0px;
      margin-left: -16px;
      margin-right: 13px;
      border-radius: 9999px;
      background: var(--ctp-mauve);
      transition: height 100ms ease;
    }
    li.active::before {
      content: "";
      height: 22px;
    }
  }
}
.custom-slide-bottom-leave-active {
  transition: all 0.3s cubic-bezier(0.75, 0, 1, 0.2);
}

.custom-slide-bottom-enter-active {
  transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
}

.custom-slide-bottom-leave-from {
  transform: translate(0, 0);
}

.custom-slide-bottom-leave-to {
  transform: translate(0, 70px);
}

.custom-slide-bottom-enter-from {
  transform: translate(0, 70px);
}

.custom-slide-bottom-enter-to {
  transform: translate(0, 0);
}
</style>
