<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <div class="mods-list">
      <div v-for="(pack, index) in resourcepacks" class="content" :key="index">
        <img v-if="pack.icon" :src="pack.icon" alt="pack icon" width="72px" height="100%" />
        <img
          v-else
          src="@/assets/images/Unknown_server.webp"
          alt="pack icon"
          width="72px"
          height="100%" />
        <div class="content-info">
          <p class="name">
            <span>{{ pack.name }}</span>
          </p>
          <p class="mod-description">{{ pack.metadata?.pack?.description }}</p>
          <span class="version" v-if="formatRange(pack)">{{ formatRange(pack) }}</span>
        </div>
        <div class="actions">
          <button class="open-folder" @click="revealItemInDir(pack.path)">
            <AppIcon name="folder" :size="14"></AppIcon>
          </button>
          <button class="delete">
            <AppIcon name="trash" :size="14"></AppIcon>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useGameContentStore } from "@/store/content";
import type { Resourcepack } from "@conic/content";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

const gameContentStore = useGameContentStore();
const resourcepacks = computed(() => gameContentStore.gameContent.resourcepacks ?? []);

function formatRange(pack: Resourcepack): string {
  const min = pack.metadata?.pack?.min_format?.[0];
  const max = pack.metadata?.pack?.max_format?.[0];
  if (min === undefined && max === undefined) return "";
  if (min === max) return `${min}`;
  if (max === undefined) return `${min}+`;
  return `${min}-${max}`;
}
</script>

<style lang="less" scoped>
.mods-list-wrapper {
  padding: 16px 32px 32px 32px;
}
.mods-list {
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
      width: calc(100% - 72px);
      border-radius: 8px;
      transition: all 200ms ease;
      p.name {
        font-size: 14px;
        width: 100%;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
      }
      p.mod-description {
        font-size: 10px;
        margin: 2px 0;
        opacity: 0.6;
        width: 100%;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
      }
      span.version {
        font-size: 9px;
        padding: 2px 6px;
        margin-right: 4px;
        border-radius: 100px;
        font-weight: 500;
        color: var(--ctp-text);
        border: 1px solid var(--ctp-sky);
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
  .content.content-disabled {
    opacity: 0.7;
    .name {
      text-decoration: line-through;
    }
  }
  .content:hover {
    .content-info {
      width: calc(100% - 88px);
      background: var(--ctp-surface1);
      transition:
        background 20ms ease,
        width 200ms ease;
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
    .download-button button {
      opacity: 1;
      transform: scale(1);
    }
    img:active ~ .download-button button {
      opacity: 0.7;
      transition: opacity 55ms ease;
    }
  }
}
</style>
