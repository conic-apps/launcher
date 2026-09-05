<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <div class="search-status" v-if="gameContentStore.loading.mods">
      <div class="loading">
        <BaseLoading :size="32" :gap="8" :strokeWidth="4"></BaseLoading>
      </div>
    </div>
    <div class="mods-list" v-else>
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
          :show="Object.keys(mods ?? {}).length === 0"
          :description="t('overlays.content.mods.localEmpty')" />
      </div>
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
            <span v-if="mod.disabled">{{ t("overlays.content.mods.disabledPrefix") }}</span>
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
          <button class="open-folder" @click="revealItemInDir(mod.path)">
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
import { useI18n } from "vue-i18n";
import { useGameContentStore } from "@/store/content";
import { ModLoader } from "@conic/content";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import ContentNotFound from "./ContentNotFound.vue";
import BaseLoading from "@/components/BaseLoading.vue";

const { t } = useI18n();

const gameContentStore = useGameContentStore();
const mods = computed(() => gameContentStore.gameContent.mods);
</script>

<style lang="less" scoped>
@import "./styles/content-card.less";

.mods-list {
  &:extend(.content-card-grid all);
}

.search-status {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
  font-size: 13px;
  color: var(--ctp-subtext0);

  .loading {
    background: var(--ctp-mantle);
    padding: 16px;
    border-radius: 8px;
  }
}
</style>
