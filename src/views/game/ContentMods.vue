<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-saves" @click="selectedSave = null">
    <div class="title">
      <AppIcon name="extension-puzzle"></AppIcon>
      <p>模组列表</p>
      <div class="select-source">
        <button><AppIcon name="file-tray-full-outline"></AppIcon></button>
        <button>
          <Modrinth fill="var(--ctp-green)" style="width: 24px; padding: 3px"></Modrinth>
        </button>
        <button>
          <CurseForge fill="var(--ctp-peach)" style="width: 24px"></CurseForge>
        </button>
      </div>
    </div>
    <div class="save-list-wrapper">
      <div class="saves-list">
        <div
          v-for="(save, folderName) in saves"
          class="content"
          :class="{
            selected: folderName === selectedSave,
            'expand-up': expandDirection === 'up',
          }"
          :key="folderName">
          <!-- <img -->
          <!--   v-if="iconCache[folderName]" -->
          <!--   :src="iconCache[folderName]" -->
          <!--   alt="world icon" -->
          <!--   width="64px" -->
          <!--   height="64px" /> -->
          <img
            src="@/assets/images/Unknown_server.webp"
            alt="world icon"
            width="64px"
            height="64px" />
          <div class="content-info" @click.stop="selectSave(folderName, $event)">
            <p class="name">{{ save.Data.LevelName }}</p>
            <p class="folder-name">{{ folderName }}</p>
            <span
              class="game-mode"
              v-if="save.Data.GameType"
              :class="{
                survival: save.Data.GameType === 0,
                creative: save.Data.GameType === 1,
                adventure: save.Data.GameType === 2,
                spectator: save.Data.GameType === 3,
              }"
              >111</span
            >
            <span class="last-played" v-if="save.Data.LastPlayed"
              ><span class="label">上次游玩: </span>111</span
            >
          </div>
          <div class="actions">
            <button>
              <AppIcon name="heart-outline" :size="14"></AppIcon>
            </button>
            <button>
              <AppIcon name="save" :size="14"></AppIcon>
            </button>
          </div>
          <div class="download-button">
            <button>
              <AppIcon name="download" :size="18"></AppIcon>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useGameContentStore } from "@/store/content";
import { computed, ref } from "vue";
import Modrinth from "@/assets/images/modrinth.svg";
import CurseForge from "@/assets/images/curseforge.svg";
import AppIcon from "@/components/AppIcon.vue";

const gameContentStore = useGameContentStore();
const saves = computed(() => gameContentStore.gameContent.saves);

const selectedSave = ref(null as string | null);

const expandDirection = ref("down" as "up" | "down");

function selectSave(folderName: string, event: MouseEvent) {
  selectedSave.value = folderName;
  const element = event.currentTarget as HTMLElement;
  const rect = element.getBoundingClientRect();
  const bottomSpace = window.innerHeight - rect.bottom;
  if (bottomSpace < 160) {
    expandDirection.value = "up";
  } else {
    expandDirection.value = "down";
  }
}
</script>

<style lang="less" scoped>
.content-saves {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  .save-list-wrapper {
    padding: 16px 32px 32px 32px;
  }
  .title {
    width: 100%;
    background: var(--ctp-mantle);
    padding: 16px 32px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 8px;
    .select-source {
      display: flex;
      align-items: center;
      border-radius: 8px;
      overflow: hidden;
      button {
        appearance: none;
        border: none;
        background: none;
        background: var(--ctp-surface0);
        width: 80px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 1px solid var(--ctp-surface1);
      }
    }
  }
}
.saves-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  justify-content: center;
  width: 100%;
  row-gap: 12px;
  column-gap: 12px;
  .content {
    display: flex;
    border-radius: 8px;
    image-rendering: pixelated;
    transform: translateX(4px);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    img {
      border: 2px solid var(--ctp-surface0);
      border-radius: 8px 0 0 8px;
      transition: opacity 200ms ease;
    }
    .content-info {
      background: var(--ctp-surface0);
      padding: 8px 12px;
      transform: translateX(-8px);
      width: calc(100% - 64px);
      border-radius: 8px;
      transition: all 200ms ease;
      p.name {
        font-size: 14px;
      }
      p.folder-name {
        font-size: 10px;
        margin: 4px 0;
        opacity: 0.6;
      }
      span.game-mode,
      span.command-enabled {
        font-size: 9px;
        padding: 2px 6px;
        border-radius: 100px;
        font-weight: 500;
        color: var(--ctp-text-inverse);
      }
      span.game-mode.survival {
        background: var(--ctp-green);
      }
      span.game-mode.creative {
        background: var(--ctp-mauve);
      }
      span.game-mode.adventure {
        background: var(--ctp-peach);
      }
      span.game-mode.spectator {
        background: var(--ctp-blue);
      }
      span.command-enabled {
        background: var(--ctp-yellow);
        margin-left: 4px;
      }
      span.last-played {
        font-size: 10px;
        margin-left: 4px;
        span.label {
          opacity: 0.8;
        }
      }
    }
    .actions {
      position: absolute;
      right: 4px;
      top: 0;
      height: 100%;
      display: flex;
      flex-direction: column;
      padding: 12px 0;
      align-items: center;
      justify-content: space-between;
      z-index: -1;
      button {
        appearance: none;
        border: none;
        background: none;
        opacity: 0;
        transform: scale(0.5);
        transition:
          opacity 200ms ease,
          transform 200ms ease;
      }
    }
    .download-button {
      position: absolute;
      left: 20px;
      top: 50%;
      transform: translateY(-50%);
      button {
        appearance: none;
        background: none;
        border: none;
        opacity: 0;
        transform: scale(0.5);
        transition:
          opacity 200ms ease,
          transform 200ms ease;
      }
    }
  }
  .content:hover {
    .content-info {
      width: calc(100% - 80px);
    }
    .actions button {
      opacity: 0.8;
      transform: scale(1);
    }
    .actions button:hover {
      opacity: 1;
    }
    .actions button:active {
      opacity: 0.9;
    }
    img {
      opacity: 0.4;
    }
    .download-button button {
      opacity: 1;
      transform: scale(1);
    }
    img:active ~ .download-button button {
      opacity: 0.7;
      transition: opacity 55ms ease;
    }
  }
  .content .extra {
    position: absolute;
    width: 100%;
    height: 100%;
    border-radius: inherit;
    z-index: -3;
    opacity: 0;
    overflow: hidden;
    transition: all 200ms ease;
    background: var(--ctp-surface0);

    .map-previewer-container {
      position: absolute;
      height: 100%;
      width: 100%;
      left: 0;
      right: 16px;
      bottom: 0;
      padding: 8px;
    }
  }
  .content.selected {
    z-index: 10;
    img {
      z-index: 9;
    }
  }
  .content.selected .extra {
    outline: 2px solid var(--ctp-blue);
    height: calc(100% + 160px);
    z-index: 11;
    opacity: 1;
  }
  .content.expand-up .extra {
    bottom: 0;
    top: unset;
  }
}
</style>
