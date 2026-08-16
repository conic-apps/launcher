<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <div class="mods-list">
      <div
        v-for="(mod, index) in mods"
        class="content"
        :class="{ 'content-disabled': mod.disabled }"
        :key="index">
        <img v-if="mod.icon" :src="mod.icon" alt="mod icon" width="72px" height="100%" />
        <img
          v-else
          src="@/assets/images/Unknown_server.webp"
          alt="world icon"
          width="72px"
          height="100%" />
        <div class="content-info">
          <p class="name">
            <span v-if="mod.disabled">[已禁用] </span>
            <span>{{ mod.name }}</span>
          </p>
          <p class="authors">by {{ mod.authors.map((authorInfo) => authorInfo.name).join(",") }}</p>
          <p class="mod-description">{{ mod.description }}</p>
          <span
            class="loader-type"
            v-if="mod.loader !== ModLoader.Unknown"
            :class="{
              fabric: mod.loader === ModLoader.Fabric,
              forge: mod.loader === ModLoader.Forge,
              quilt: mod.loader === ModLoader.Quilt,
              neoforge: mod.loader === ModLoader.NeoForge,
              liteloader: mod.loader === ModLoader.LiteLoader,
            }"
            >{{ mod.loader.charAt(0).toUpperCase() + mod.loader.slice(1) }}</span
          >
          <span class="version" v-if="mod.version">{{ mod.version }}</span>
        </div>
        <div class="actions">
          <button class="open-folder">
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
import { ModLoader } from "@conic/content";

const gameContentStore = useGameContentStore();
const mods = computed(() => gameContentStore.gameContent.mods);
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
      p.authors {
        width: 100%;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
        font-size: 11px;
        opacity: 0.9;
        margin: 2px 0;
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
      span.loader-type,
      span.version {
        font-size: 9px;
        padding: 2px 6px;
        margin-right: 4px;
        border-radius: 100px;
        font-weight: 500;
        color: var(--ctp-text-inverse);
      }
      span.loader-type.fabric {
        background: var(--ctp-yellow);
      }
      span.loader-type.forge {
        background: var(--ctp-blue);
      }
      span.loader-type.neoforge {
        background: var(--ctp-peach);
      }
      span.loader-type.quilt {
        background: var(--ctp-mauve);
      }
      span.loader-type.liteloader {
        background: var(--ctp-yellow);
      }
      span.version {
        border: 1px solid var(--ctp-sky);
        color: var(--ctp-text);
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
