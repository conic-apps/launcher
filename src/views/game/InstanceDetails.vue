<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-details">
    <tabs
      v-if="instanceStore.currentInstance.installed"
      :tabs="
        components.map((n) => {
          return i18n.t(n.name);
        })
      "
      :active="activeTab"
      :icons="
        components.map((n) => {
          return n.icon;
        })
      "
      @choose-tab="chooseTab">
    </tabs>
    <Transition :name="transitionName" mode="out-in">
      <component
        :is="currentComponent"
        style="padding: 16px 8px; width: 100%; height: fit-content"></component>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { computed, markRaw, ref } from "vue";
import Tabs from "@/components/Tabs.vue";
import Info from "./Info.vue";
import Worlds from "./Worlds.vue";
import Mods from "./Mods.vue";
import Packs from "./Packs.vue";
import Settings from "./Settings.vue";
import { useI18n } from "vue-i18n";
import { useInstanceStore } from "@/store/instance";

const i18n = useI18n();
const instanceStore = useInstanceStore();

const components = ref([
  {
    name: "game.assets.info",
    icon: "about",
    component: markRaw(Info),
  },
  {
    name: "game.assets.worlds",
    icon: "save",
    component: markRaw(Worlds),
  },
  {
    name: "game.assets.mods",
    icon: "extension-puzzle",
    component: markRaw(Mods),
  },
  {
    name: "game.assets.packs",
    icon: "folder",
    component: markRaw(Packs),
  },
  {
    name: "game.assets.settings",
    icon: "settings",
    component: markRaw(Settings),
  },
]);
const activeTab = ref(0);

const transitionName = ref("slide-left");

function chooseTab(tab: number) {
  if (activeTab.value < tab) {
    transitionName.value = "slide-left";
  } else {
    transitionName.value = "slide-right";
  }
  activeTab.value = tab;
}
const currentComponent = computed(() => {
  return components.value[activeTab.value].component;
});
</script>

<style lang="less" scoped>
.instance-details {
  margin-top: 14px;
  width: 100%;
  overflow-x: hidden;

  > div {
    display: flex;
    width: 100%;
  }

  > div.first-row {
    margin-right: 5px;
  }

  > div.second-row {
    margin-left: 5px;
  }
}
</style>
