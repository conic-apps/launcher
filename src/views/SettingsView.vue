<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="settings-view" ref="root" style="opacity: 0">
    <div class="column-left">
      <ul class="settings-menu">
        <li
          @click="scrollToSection(index)"
          :class="{ active: activeSection == index }"
          v-for="(item, index) in components"
          :key="item.icon"
          style="opacity: 0">
          <AppIcon :name="item.icon" :size="16"></AppIcon><span>{{ item.name }}</span>
        </li>
      </ul>
    </div>
    <div class="column-right">
      <ScrollView ref="scrollViewRef" @scroll="onScroll">
        <div class="settings-content">
          <section v-for="(item, index) in components" :key="index" class="settings-section">
            <h2 class="settings-section-title">{{ item.name }}</h2>
            <component :is="item.component"></component>
          </section>
        </div>
      </ScrollView>
    </div>
  </div>
</template>

<script setup lang="ts">
import { type Component, computed, markRaw, nextTick, onMounted, ref, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
import GeneralSettings from "./settings/SettingsGeneral.vue";
import gsap from "gsap";
import GameSettings from "./settings/SettingsGame.vue";
import JvmSettings from "./settings/SettingsJVM.vue";
import AppearanceSettings from "./settings/SettingsAppearance.vue";
import DownloadSettings from "./settings/SettingsDownload.vue";
import AccessibilitySettings from "./settings/SettingsAccessibility.vue";
import AboutSettings from "./settings/SettingsAbout.vue";
import AudioSettings from "./settings/SettingsAudio.vue";
import AppIcon from "@/components/AppIcon.vue";
import ScrollView from "@/components/ScrollView.vue";

const components = computed<{ name: string; icon: string; component: Component }[]>(() => [
  {
    name: t("settings.tabs.general"),
    icon: "house",
    component: markRaw(GeneralSettings),
  },
  {
    name: t("settings.tabs.launchOptions"),
    icon: "gamepad",
    component: markRaw(GameSettings),
  },
  {
    name: t("settings.tabs.jvm"),
    icon: "java",
    component: markRaw(JvmSettings),
  },
  {
    name: t("settings.tabs.appearance"),
    icon: "palette",
    component: markRaw(AppearanceSettings),
  },
  {
    name: t("settings.tabs.audio"),
    icon: "musical-notes",
    component: markRaw(AudioSettings),
  },
  {
    name: t("settings.tabs.download"),
    icon: "globe",
    component: markRaw(DownloadSettings),
  },
  {
    name: t("settings.tabs.accessibility"),
    icon: "accessibility",
    component: markRaw(AccessibilitySettings),
  },
  {
    name: t("settings.tabs.about"),
    icon: "about",
    component: markRaw(AboutSettings),
  },
]);
const activeSection = ref(0);
const rootRef = useTemplateRef("root");
const scrollViewRef = useTemplateRef("scrollViewRef");
const sectionElements: HTMLElement[] = [];

onMounted(async () => {
  await nextTick();
  const root = rootRef.value;
  if (!root) return;
  sectionElements.push(...Array.from(root.querySelectorAll<HTMLElement>(".settings-section")));
  const sidebarItems = Array.from(root.querySelectorAll<HTMLElement>(".settings-menu > li"));
  const settingItems = Array.from(root.querySelectorAll<HTMLElement>(".setting-item"));
  gsap.set(settingItems, { opacity: 0, scale: 0.9 });
  const intro = gsap.timeline();
  intro.fromTo(root, { opacity: 0 }, { opacity: 1, duration: 0.333, ease: "power3.out" });
  intro.fromTo(
    sidebarItems,
    { opacity: 0, x: -20 },
    { opacity: 1, x: 0, duration: 0.33, stagger: 0.03, ease: "power3.out" },
    "<",
  );
  intro.fromTo(
    settingItems,
    { opacity: 0, scale: 0.9 },
    {
      opacity: 1,
      scale: 1,
      duration: 0.33,
      stagger: 0.03,
      ease: "power3.out",
      clearProps: "transform",
    },
    "<0.03",
  );
});

function scrollToSection(index: number) {
  const scrollView = scrollViewRef.value;
  const section = sectionElements[index];
  const wrapper = scrollView?.getWrapper();
  if (!scrollView || !section || !wrapper) return;
  const target =
    wrapper.scrollTop +
    section.getBoundingClientRect().top -
    wrapper.getBoundingClientRect().top -
    16;
  scrollView.scrollTo(target, true);
}

function onScroll() {
  const wrapper = scrollViewRef.value?.getWrapper();
  if (!wrapper || sectionElements.length === 0) return;
  const wrapperTop = wrapper.getBoundingClientRect().top;
  let active = 0;
  for (let i = 0; i < sectionElements.length; i++) {
    if (sectionElements[i].getBoundingClientRect().top - wrapperTop <= 100) {
      active = i;
    }
  }
  if (wrapper.scrollTop + wrapper.clientHeight >= wrapper.scrollHeight - 1) {
    active = sectionElements.length - 1;
  }
  activeSection.value = active;
}
</script>

<style lang="less" scoped>
.settings-view {
  width: 100%;
  height: 100%;
  display: flex;
  background: rgba(var(--ctp-base-rgb), 0.6);
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
  }

  .settings-content {
    padding: 24px 24px 24px 0;
    padding-left: 16px;
  }

  .settings-section {
    margin-bottom: 24px;
  }

  .settings-section-title {
    margin: 0 0 18px 8px;
    font-size: 18px;
    font-weight: normal;
    color: var(--ctp-text);
  }

  .settings-section:last-child {
    margin-bottom: 0;
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
      opacity: 0;
      margin-left: -16px;
      margin-right: 13px;
      border-radius: 9999px;
      background: var(--ctp-mauve);
      transition:
        height 100ms ease,
        opacity 100ms ease;
    }
    li.active::before {
      content: "";
      height: 22px;
      opacity: 1;
    }
  }
}
</style>
