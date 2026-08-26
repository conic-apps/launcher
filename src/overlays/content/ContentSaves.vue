<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-saves" @click="selectedSave = null">
    <div
      style="
        display: flex;
        align-items: center;
        justify-content: center;
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        pointer-events: none;
      ">
      <ContentNotFound
        :show="Object.keys(saves ?? {}).length === 0"
        :description="t('overlays.content.saves.empty')" />
    </div>
    <ScrollView>
      <div class="title">
        <AppIcon name="save"></AppIcon>
        <p>{{ t("overlays.content.saves.title") }}</p>
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
            <img
              v-if="iconCache[folderName]"
              :src="iconCache[folderName]"
              alt="world icon"
              width="64px"
              height="64px" />
            <img
              v-else
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
                >{{ formatGameType(save.Data.GameType) }}</span
              >
              <span class="command-enabled" v-if="save.Data.allowCommands">{{
                t("overlays.content.saves.cheats")
              }}</span>
              <span class="last-played" v-if="save.Data.LastPlayed"
                ><span class="label">{{ t("overlays.content.saves.lastPlayed") }}</span
                >{{ formatLastPlayed(save.Data.LastPlayed, timeFormatter) }}</span
              >
            </div>
            <div class="actions">
              <button class="open-folder" @click.stop="openSaveFolder(folderName)">
                <AppIcon name="folder" :size="14"></AppIcon>
              </button>
              <button class="delete" @click.stop="askDeleteSave(folderName)">
                <AppIcon name="trash" :size="14"></AppIcon>
              </button>
            </div>
            <div class="play-button">
              <button class="play">
                <AppIcon name="play" :size="18"></AppIcon>
              </button>
            </div>
            <!-- FIXME: Disable page scroll when scale -->
            <div class="extra" @click.stop>
              <div class="map-previewer-container">
                <WorldMap
                  v-if="selectedSave === folderName && instanceStore.currentInstance"
                  :instance-id="instanceStore.currentInstance.id"
                  :show-cursor-coords="true"
                  :folder-name="folderName"
                  cursor-coordinates-position="bottom-center"
                  :center-x="saveSpawnX(save)"
                  :center-z="saveSpawnZ(save)"></WorldMap>
              </div>
            </div>
          </div>
        </div>
      </div>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import ScrollView from "@/components/ScrollView.vue";
import WorldMap from "@/components/WorldMap.vue";
import { useGameContentStore } from "@/store/content";
import { useDialogStore } from "@/store/dialog";
import { useInstanceStore } from "@/store/instance";
import { getSaveIcon, getSavePath, type Level } from "@conic/content";
import { formatLastPlayed } from "@conic/instance";
import { invoke } from "@tauri-apps/api/core";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ContentNotFound from "./ContentNotFound.vue";

const { t } = useI18n();

const timeFormatter = {
  get justNow() {
    return t("game.time.justNow");
  },
  hoursAgo: (hours: number) => t("game.time.hoursAgo", hours),
  get yesterday() {
    return t("game.time.yesterday");
  },
  monthDay: (month: number, day: number) => t("game.time.monthDay", { month, day }),
  yearMonthDay: (year: number, month: number, day: number) =>
    t("game.time.yearMonthDay", { year, month, day }),
};

const gameContentStore = useGameContentStore();
const dialogStore = useDialogStore();
const instanceStore = useInstanceStore();
const saves = computed(() => gameContentStore.gameContent.saves);

const selectedSave = ref(null as string | null);

const iconCache = ref({} as Record<string, string>);

watch(
  () => gameContentStore.gameContent.saves,
  async (saves) => {
    if (!saves) {
      return;
    }
    const promises = Object.keys(saves).map(async (key) => {
      try {
        if (!instanceStore.currentInstance) {
          throw "currentInstance is null";
        }
        iconCache.value[key] = await getSaveIcon(instanceStore.currentInstance.id, key);
      } catch (error) {
        console.error(error);
      }
    });
    await Promise.allSettled(promises);
  },
  { immediate: true },
);

function formatGameType(gameType: number) {
  if (gameType === 0) {
    return t("overlays.content.saves.gameType.survival");
  } else if (gameType === 1) {
    return t("overlays.content.saves.gameType.creative");
  } else if (gameType === 2) {
    return t("overlays.content.saves.gameType.adventure");
  } else if (gameType === 3) {
    return t("overlays.content.saves.gameType.spectator");
  } else {
    return null;
  }
}

function readSpawnPos(pos: unknown): number[] {
  if (Array.isArray(pos)) return pos;
  if (pos && typeof pos === "object") {
    const wrapped = (pos as { __fastnbt_int_array?: unknown }).__fastnbt_int_array;
    if (Array.isArray(wrapped)) return wrapped;
  }
  return [];
}

function saveSpawnX(save: Level): number | undefined {
  return readSpawnPos(save.Data.spawn?.pos)[0];
}

function saveSpawnZ(save: Level): number | undefined {
  return readSpawnPos(save.Data.spawn?.pos)[2];
}

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

async function openSaveFolder(folderName: string) {
  if (!instanceStore.currentInstance) {
    throw "currentInstance is null";
  }
  invoke("open_path", {
    path: await getSavePath(instanceStore.currentInstance.id, folderName),
  });
}

function askDeleteSave(folderName: string) {
  const save = saves.value?.[folderName];
  dialogStore.confirmDeleteSave.folderName = folderName;
  dialogStore.confirmDeleteSave.levelName = save?.Data.LevelName ?? folderName;
  dialogStore.confirmDeleteSave.visible = true;
}
</script>

<style lang="less" scoped>
.content-saves {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;
  .save-list-wrapper {
    padding: 16px 32px 32px 32px;
  }
  .title {
    width: 100%;
    background: var(--ctp-mantle);
    height: 52px;
    padding: 0 32px;
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
