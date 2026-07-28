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
    <Transition name="custom-slide-bottom">
      <div class="back-button" v-if="showBackButton">
        <button @click="back">
          <div>
            <AppIcon name="arrow-back-outline"></AppIcon>
          </div>
          <span>返回</span>
        </button>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { type Component, markRaw, onMounted, ref, shallowRef } from "vue";
import GeneralSettings from "./settings/GeneralSettings.vue";
import GameSettings from "./settings/GameSettings.vue";
import JvmSettings from "./settings/JvmSettings.vue";
import AdvanceSettings from "./settings/AdvanceSettings.vue";
import AppearanceSettings from "./settings/AppearanceSettings.vue";
import DownloadSettings from "./settings/DownloadSettings.vue";
import AccessibilitySettings from "./settings/AccessibilitySettings.vue";
import AboutSettings from "./settings/AboutSettings.vue";
import AppIcon from "@/components/AppIcon.vue";
import { useNavigationStore } from "@/store/navigation";

const navigationStore = useNavigationStore();

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
    name: "settings.advance.sidebar",
    icon: "build",
    component: markRaw(AdvanceSettings),
  },
  {
    name: "settings.appearance.sidebar",
    icon: "palette",
    component: markRaw(AppearanceSettings),
  },
  {
    name: "settings.download.sidebar",
    icon: "cloud-download",
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

const showBackButton = ref(false);

onMounted(() => {
  showBackButton.value = true;
});
function back() {
  showBackButton.value = false;
  setTimeout(() => {
    navigationStore.back();
  }, 300);
}
</script>

<style lang="less" scoped>
.settings-view {
  width: 100%;
  height: 100%;
  display: flex;
  .column-left,
  .column-right {
    height: 100%;
  }

  .column-left {
    width: 200px;
    flex-shrink: 0;
    padding: 30px 0px 16px 24px;
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
      background: rgba(255, 255, 255, 0.1);
    }
    li.active {
      background: rgba(255, 255, 255, 0.16);
    }
    li::before {
      content: "";
      width: 3px;
      height: 0px;
      margin-left: -16px;
      margin-right: 13px;
      border-radius: 9999px;
      background: rgba(255, 255, 255, 0.8);
      transition: height 100ms ease;
    }
    li.active::before {
      content: "";
      height: 22px;
    }
  }
  .back-button {
    position: fixed;
    bottom: 32px;
    left: 32px;
    button {
      appearance: none;
      background: var(--ctp-latte-lavender);
      border: none;
      height: 48px;
      padding-right: 26px;
      border-radius: 1000px;
      display: flex;
      align-items: center;
      transition: all 0.3s ease;
      transform: scale(1);
      font-size: 14px;
      div {
        background: #ffffff3f;
        border-radius: 100px;
        width: 40px;
        height: 40px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-left: 6px;
      }
      span {
        margin-left: 10px;
      }
      &:active {
        transition: all 0.3s cubic-bezier(0, 0.75, 0.2, 1);
        transform: scale(0.97);
      }
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
