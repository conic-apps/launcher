<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <Transition name="music-overlay">
    <div class="music-overlay" v-if="music.panelOpen" @pointerdown.self="music.closePanel()">
      <div class="music-card" @pointerdown.stop>
        <div class="track-info">
          <span class="track-name">{{
            music.currentTrack?.name ?? t("app.musicplayer.noMusic")
          }}</span>
        </div>
        <div class="progress-area">
          <span class="time">{{ formatTime(music.currentTime) }}</span>
          <div
            class="progress-bar"
            ref="progressBarRef"
            @pointerdown="onProgressPointerDown"
            :title="t('app.musicplayer.progress')">
            <div class="progress-fill" :style="{ width: `${music.progress * 100}%` }"></div>
          </div>
          <span class="time">{{ formatTime(music.duration) }}</span>
        </div>
        <div class="controls">
          <div class="controls-group controls-left">
            <button
              class="player-btn"
              :class="{ active: music.shuffle }"
              @click="music.toggleShuffle()"
              :title="t('app.musicplayer.shuffle')">
              <AppIcon name="shuffle" :size="18" />
            </button>
            <button
              class="player-btn"
              :class="{ active: music.repeat }"
              @click="music.cycleRepeat()"
              :title="t('app.musicplayer.repeat')">
              <AppIcon name="repeat" :size="18" />
            </button>
          </div>
          <div class="controls-group controls-center">
            <button class="player-btn" @click="music.prev()" :title="t('app.musicplayer.prev')">
              <AppIcon name="play-skip-back" :size="18" />
            </button>
            <button
              class="player-btn play-btn"
              @click="music.togglePlay()"
              :title="music.isPlaying ? t('app.musicplayer.pause') : t('app.musicplayer.play')">
              <AppIcon v-if="music.isPlaying" name="pause-circle-outline" :size="24" />
              <AppIcon v-else name="play-circle-outline" :size="24" />
            </button>
            <button class="player-btn" @click="music.next()" :title="t('app.musicplayer.next')">
              <AppIcon name="play-skip-forward" :size="18" />
            </button>
          </div>
          <div class="controls-group controls-right">
            <button
              class="player-btn"
              @click="openMusicFolder"
              :title="t('app.musicplayer.openFolder')">
              <AppIcon name="folder" :size="18" />
            </button>
            <button
              class="player-btn"
              :class="{ active: showPlaylist }"
              @click="showPlaylist = !showPlaylist"
              :title="t('app.musicplayer.playlist')">
              <AppIcon name="list" :size="18" />
            </button>
          </div>
        </div>
      </div>
      <Transition name="playlist-fade">
        <div class="playlist-card" v-if="showPlaylist" @pointerdown.stop>
          <div class="playlist-scroll">
            <ScrollView>
              <div class="playlist-items">
                <button
                  class="playlist-item"
                  v-for="(track, index) in music.tracks"
                  :key="track.path"
                  :class="{ playing: index === music.currentIndex }"
                  @click="onSelectTrack(index)">
                  <span class="playlist-item-name">{{ track.name }}</span>
                </button>
                <p v-if="music.tracks.length === 0" class="playlist-empty">
                  {{ t("app.musicplayer.empty") }}
                </p>
              </div>
            </ScrollView>
          </div>
        </div>
      </Transition>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, useTemplateRef, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getDataLocation } from "@conic/folder";
import ScrollView from "@/components/ScrollView.vue";
import { useConfigStore } from "@/store/config";
import { useMusicStore } from "@/store/music";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const config = useConfigStore();
const music = useMusicStore();
const showPlaylist = ref(false);
const progressBarRef = useTemplateRef("progressBarRef");

music.init();
music.restoreSession();

watch(
  () => config.music.enabled,
  (enabled) => {
    if (!enabled) {
      music.pause();
      music.closePanel();
    }
  },
);

async function openMusicFolder() {
  const dataLocation = await getDataLocation();
  await invoke("open_path", { path: dataLocation.music });
}

function formatTime(time: number): string {
  if (!Number.isFinite(time) || time < 0) {
    return "00:00";
  }
  const totalSeconds = Math.floor(time);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function onSelectTrack(index: number) {
  music.playIndex(index);
}

function onProgressPointerDown(event: PointerEvent) {
  const bar = progressBarRef.value;
  if (!bar || music.duration <= 0) {
    return;
  }
  const updateProgress = (pointerEvent: PointerEvent) => {
    const rect = bar.getBoundingClientRect();
    const ratio = Math.min(Math.max((pointerEvent.clientX - rect.left) / rect.width, 0), 1);
    music.seekRatio(ratio);
  };
  updateProgress(event);
  const onPointerMove = (pointerEvent: PointerEvent) => {
    updateProgress(pointerEvent);
  };
  const onPointerUp = () => {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
  };
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
}
</script>

<style lang="less" scoped>
.music-overlay {
  position: fixed;
  top: 44px;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 100;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 12px;
  padding: 16px;
  overflow-y: auto;
  background: rgba(0, 0, 0, 0.45);
}

.music-card {
  display: flex;
  flex-direction: column;
  width: 340px;
  padding: 16px;
  border-radius: 14px;
  background: var(--ctp-base);
  border: 1px solid rgba(var(--ctp-overlay0-rgb), 0.5);
  box-shadow: 0 4px 24px 0 rgba(0, 0, 0, 0.3);

  .track-info {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: 0;

    .track-name {
      font-size: 16px;
      font-weight: 600;
      margin-bottom: 18px;
      color: var(--ctp-text);
      text-align: center;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  .progress-area {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;

    .time {
      font-size: 11px;
      color: var(--ctp-subtext0);
      font-variant-numeric: tabular-nums;
      flex-shrink: 0;
    }

    .progress-bar {
      flex: 1;
      height: 4px;
      border-radius: 4px;
      background: rgba(var(--ctp-surface2-rgb), 0.8);
      position: relative;

      .progress-fill {
        height: 100%;
        border-radius: 4px;
        background: var(--ctp-mauve);
      }
    }
  }

  .controls {
    display: flex;
    align-items: center;

    .controls-group {
      display: flex;
      align-items: center;
    }

    .controls-left {
      flex: 1;
      justify-content: flex-start;
      gap: 2px;
    }

    .controls-center {
      flex: 1;
      justify-content: center;
      gap: 2px;
    }

    .controls-right {
      flex: 1;
      justify-content: flex-end;
      gap: 2px;
    }

    .player-btn {
      appearance: none;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 4px;
      width: 34px;
      height: 34px;
      border: none;
      border-radius: 10px;
      background: transparent;
      color: var(--ctp-subtext0);
      transition: all 120ms ease;
      position: relative;

      &:hover {
        background: rgba(var(--ctp-surface2-rgb), 0.7);
        color: var(--ctp-text);
      }

      &:active {
        background: rgba(var(--ctp-overlay0-rgb), 0.7);
      }

      &.active {
        color: var(--ctp-mauve);

        :deep(svg path) {
          stroke: var(--ctp-mauve);
        }
      }

      &.play-btn {
        width: 40px;
        height: 40px;
        color: var(--ctp-mauve);

        &:hover {
          background: rgba(var(--ctp-surface2-rgb), 0.7);
          color: var(--ctp-mauve);
        }
      }
    }
  }
}

.playlist-card {
  width: 340px;
  max-height: 320px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  position: relative;
  padding: 8px;
  border-radius: 14px;
  background: var(--ctp-base);
  border: 1px solid rgba(var(--ctp-overlay0-rgb), 0.5);
  box-shadow: 0 4px 24px 0 rgba(0, 0, 0, 0.3);
  flex-shrink: 0;

  .playlist-scroll {
    flex: 1 1 auto;
    min-height: 0;
    position: relative;

    :deep(.wrapper) {
      height: 100%;
    }
  }

  .playlist-items {
    padding: 4px 0 8px;
  }

  .playlist-item {
    appearance: none;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 8px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--ctp-subtext0);
    font-size: 12px;
    text-align: left;

    &:hover {
      background: rgba(var(--ctp-surface1-rgb), 0.7);
      color: var(--ctp-text);
    }

    &.playing {
      color: var(--ctp-blue);
    }

    .playlist-item-name {
      color: inherit;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }

  .playlist-empty {
    padding: 12px;
    margin: 0;
    font-size: 12px;
    color: var(--ctp-overlay0);
    text-align: center;
  }
}

.music-overlay-enter-active,
.music-overlay-leave-active {
  transition: opacity 150ms ease;
}

.music-overlay-enter-active .music-card,
.music-overlay-leave-active .music-card {
  transition: transform 150ms ease;
}

.music-overlay-enter-from,
.music-overlay-leave-to {
  opacity: 0;
}

.music-overlay-enter-from .music-card,
.music-overlay-leave-to .music-card {
  transform: translateY(-8px);
}

.playlist-fade-enter-active,
.playlist-fade-leave-active {
  transition: all 120ms ease;
}

.playlist-fade-enter-from,
.playlist-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
