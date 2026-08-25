<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <div class="mods-list">
      <div
        style="
          display: flex;
          align-items: center;
          justify-content: center;
          position: absolute;
          top: 0;
          left: 0;
          width: 100%;
          pointer-events: none;
          height: 100%;
        ">
        <ContentNotFound
          :show="resourcepacks.length === 0"
          description="考虑从互联网下载一些资源包"></ContentNotFound>
      </div>
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
import ContentNotFound from "./ContentNotFound.vue";

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
@import "./styles/content-card.less";

.mods-list-wrapper {
  padding: 16px 32px 32px 32px;
}

.mods-list {
  &:extend(.content-card-grid all);
}
</style>
