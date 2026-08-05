<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-data-view" data-tauri-drag-region>
    <div class="dock">
      <button :class="{ active: true }">
        <AppIcon name="save"></AppIcon>
      </button>
      <button>
        <AppIcon name="extension-puzzle"></AppIcon>
      </button>
      <button>
        <AppIcon name="folder"></AppIcon>
      </button>
      <button>
        <AppIcon name="images-outline"></AppIcon>
      </button>
    </div>
    <div class="row-1">
      <div class="current-instance-info">
        <p class="title" v-if="currentInstance.id === LATEST_RELEASE_INSTANCE_ID">
          {{ $t("game.latestRelease") }}
        </p>
        <p class="title" v-else-if="currentInstance.id === LATEST_SNAPSHOT_INSTANCE_ID">
          {{ $t("game.latestSnapshot") }}
        </p>
        <p class="title" v-else>{{ currentInstance.config.name }}</p>
        <div style="display: flex">
          <p class="version">
            <span> Minecraft 版本 </span>
            <span>{{ currentInstance.config.runtime.minecraft }}</span>
          </p>
          <div
            class="line"
            v-if="
              currentInstance.config.runtime.mod_loader_type &&
              currentInstance.config.runtime.mod_loader_version
            "></div>
          <p
            class="version"
            v-if="
              currentInstance.config.runtime.mod_loader_type &&
              currentInstance.config.runtime.mod_loader_version
            ">
            <span> {{ currentInstance.config.runtime.mod_loader_type }} 版本 </span>
            <span>{{ currentInstance.config.runtime.mod_loader_version }}</span>
          </p>
          <button class="action">
            <AppIcon name="folder"></AppIcon>
          </button>
        </div>
      </div>
    </div>
    <div class="container">
      <!-- <component></component> -->
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { useInstanceStore } from "@/store/instance";
import { LATEST_RELEASE_INSTANCE_ID, LATEST_SNAPSHOT_INSTANCE_ID } from "@conic/instance";
import { computed } from "vue";
// import { computed, markRaw, ref, shallowRef } from "vue";
// import GameDataWorlds from "./gameData/GameDataWorlds.vue";
// import GameDataMods from "./gameData/GameDataMods.vue";
// import GameDataResourcepacks from "./gameData/GameDataResourcepacks.vue";
// import GameDataScreenshots from "./gameData/GameDataScreenshots.vue";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => instanceStore.currentInstance);

// const props = defineProps<{
//   currentComponent: "mods" | "worlds" | "resourcepacks" | "screenshots";
// }>();

// const components = ref({
//   worlds: markRaw(GameDataWorlds),
//   mods: markRaw(GameDataMods),
//   resourcepacks: markRaw(GameDataResourcepacks),
//   screenshots: markRaw(GameDataScreenshots),
// });
// const currentComponent = shallowRef(components.value.mods);
</script>

<style lang="less" scoped>
.game-data-view {
  width: 100%;
  height: 100%;
  padding: 32px;
  position: relative;
  .dock {
    position: absolute;
    display: flex;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--ctp-base);
    padding: 8px;
    border-radius: 8px;
    button {
      appearance: none;
      border: none;
      background: none;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      width: 40px;
      height: 40px;
      border-radius: 4px;
      transition: all 100ms ease;
      &:hover {
        background: var(--ctp-surface0);
      }
      &:active {
        background: var(--ctp-surface1);
      }
    }
  }
  .current-instance-info {
    display: flex;
    width: 100%;
    justify-content: space-between;
    p.title {
      font-size: 24px;
    }
  }
  .row-1 {
    display: flex;
    align-items: center;

    p.version {
      font-size: 12px;
      display: flex;
      flex-direction: column;
      align-items: initial;
      width: fit-content;
      padding: 2px 4px;

      :first-child {
        opacity: 0.8;
        font-size: 12px;
      }

      :last-child {
        margin-top: 2px;
        font-size: 15px;
      }
    }
    button.action {
      appearance: none;
      border: none;
      color: var(--ctp-text);
      width: 32px;
      height: 32px;
      border-radius: 100px;
      display: flex;
      align-items: center;
      justify-content: center;
      margin-left: 16px;
      background: none;
      transition:
        background 100ms ease,
        transform 100ms ease;

      &:hover {
        background: var(--ctp-surface1);
      }

      &:active {
        transform: scale(0.9);
        background: var(--ctp-surface0);
      }

      &:last-child {
        margin-right: 0;
      }
    }

    div.line {
      width: 1px;
      height: 26px;
      background: var(--ctp-surface2);
      margin: 0px 8px;
    }
  }
}
</style>
