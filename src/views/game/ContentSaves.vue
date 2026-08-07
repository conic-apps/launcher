<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-saves">
    <div class="title">
      <AppIcon name="save"></AppIcon>
      <p>存档列表</p>
      <p style="margin-left: auto; font-size: 13px; opacity: 0.8">浏览:</p>
      <BaseSelect
        :options="['local', 'modrinth', 'curseforge']"
        :displayName="['本地存档', 'Modrinth 数据包', 'CurseForge 数据包']"></BaseSelect>
    </div>
    <div class="save-list-wrapper">
      <div class="saves-list">
        <div v-for="(save, folderName) in saves" class="content" :key="folderName">
          <img
            v-if="iconCache[folderName]"
            :src="iconCache[folderName]"
            alt="world icon"
            width="64px"
            height="64px" />
          <div class="content-info">
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
              >{{ formatGameType(save.Data.GameType) }}</span
            >
            <span class="command-enabled" v-if="save.Data.allowCommands">作弊</span>
            <span class="last-played" v-if="save.Data.LastPlayed"
              ><span class="label">上次游玩: </span
              >{{ formatLastPlayed(save.Data.LastPlayed, zhCN) }}</span
            >
          </div>
          <div class="actions">
            <button class="open-folder">
              <AppIcon name="folder" :size="14"></AppIcon>
            </button>
            <button class="delete">
              <AppIcon name="trash" :size="14"></AppIcon>
            </button>
          </div>
          <div class="play-button">
            <button class="play">
              <AppIcon name="play" :size="18"></AppIcon>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import BaseSelect from "@/components/BaseSelect.vue";
import { useGameContentStore } from "@/store/content";
import { useInstanceStore } from "@/store/instance";
import { getSaveIcon } from "@conic/content";
import { formatLastPlayed, formatPlayTime, zhCN } from "@conic/instance";
import { computed, ref, watch } from "vue";

const gameContentStore = useGameContentStore();
const instanceStore = useInstanceStore();
const saves = computed(() => gameContentStore.gameContent.saves);

let iconCache = ref({} as Record<string, string>);

watch(
  () => gameContentStore.gameContent.saves,
  async (saves) => {
    console.log(2);
    if (!saves) {
      console.log(3);
      return;
    }
    const promises = Object.keys(saves).map(async (key) => {
      console.log(1);
      try {
        iconCache.value[key] = await getSaveIcon(instanceStore.currentInstance.id, key);
      } catch (error) {
        console.log(error);
      }
      console.log(iconCache);
    });
    console.log(111);
    await Promise.allSettled(promises);
  },
  { immediate: true },
);

function formatGameType(gameType: number) {
  if (gameType === 0) {
    return "生存";
  } else if (gameType === 1) {
    return "创造";
  } else if (gameType === 2) {
    return "冒险";
  } else if (gameType === 3) {
    return "旁观";
  } else {
    return null;
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
    overflow: hidden;
    image-rendering: pixelated;
    transform: translateX(4px);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    position: relative;
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
    .play-button {
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
    .play-button button {
      opacity: 1;
      transform: scale(1);
    }
    img:active ~ .play-button button {
      opacity: 0.7;
      transition: opacity 55ms ease;
    }
  }
}
</style>
