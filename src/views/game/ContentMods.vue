<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-mods">
    <ScrollView>
      <div class="title">
        <AppIcon name="extension-puzzle"></AppIcon>
        <p>模组列表</p>
        <!-- FIXME: reverse lavender color, button hover -->
        <div class="select-source">
          <button :class="{ active: currentView === 'local' }" @click="currentView = 'local'">
            <AppIcon name="file-tray-full-outline"></AppIcon>
          </button>
          <button :class="{ active: currentView === 'modrinth' }" @click="currentView = 'modrinth'">
            <Modrinth fill="var(--ctp-green)" style="width: 24px; padding: 3px"></Modrinth>
          </button>
          <button
            :class="{ active: currentView === 'curseforge' }"
            @click="currentView = 'curseforge'">
            <CurseForge fill="var(--ctp-peach)" style="width: 24px"></CurseForge>
          </button>
        </div>
      </div>
      <div class="mods-list-wrapper" v-if="currentView === 'local'">
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
              <p class="authors">
                by {{ mod.authors.map((authorInfo) => authorInfo.name).join(",") }}
              </p>
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
      <div class="mods-list-wrapper" v-if="currentView === 'modrinth'">
        <div v-if="modrinthSearchResult === null">Searching</div>
        <div class="mods-list" v-else>
          <div v-for="(mod, index) in modrinthSearchResult.hits" class="content" :key="index">
            <img
              v-if="mod.icon_url"
              :src="mod.icon_url"
              alt="mod icon"
              width="72px"
              height="100%" />
            <img
              v-else
              src="@/assets/images/Unknown_server.webp"
              alt="world icon"
              width="72px"
              height="100%" />
            <div class="content-info">
              <p class="name">
                <span>{{ mod.title }}</span>
              </p>
              <p class="authors">by {{ mod.author }}</p>
              <p class="mod-description">{{ mod.description }}</p>
              <span
                class="loader-type fabric"
                v-if="mod.categories && mod.categories.find((category) => category === 'fabric')"
                >Fabric</span
              >
              <span
                class="loader-type forge"
                v-if="mod.categories && mod.categories.find((category) => category === 'forge')"
                >Forge</span
              >
              <span
                class="loader-type quilt"
                v-if="mod.categories && mod.categories.find((category) => category === 'quilt')"
                >Quilt</span
              >
              <span
                class="loader-type neoforge"
                v-if="mod.categories && mod.categories.find((category) => category === 'neoforge')"
                >Neoforge</span
              >
              <!-- <span class="version" v-if="mod.version">{{ mod.version }}</span> -->
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
      <div class="mods-list-wrapper" v-if="currentView === 'curseforge'">
        <div v-if="curseForgeSearchResult === null || !curseForgeSearchResult.data">Searching</div>
        <div class="mods-list" v-else>
          <div v-for="(mod, index) in curseForgeSearchResult.data" class="content" :key="index">
            <img
              v-if="mod.logo.url"
              :src="mod.logo.url"
              alt="mod icon"
              width="72px"
              height="100%" />
            <img
              v-else
              src="@/assets/images/Unknown_server.webp"
              alt="world icon"
              width="72px"
              height="100%" />
            <div class="content-info">
              <p class="name">
                <span>{{ mod.name }}</span>
              </p>
              <p class="authors">
                by {{ mod.authors.map((authorInfo) => authorInfo.name).join(",") }}
              </p>
              <p class="mod-description">{{ mod.summary }}</p>
              <!-- <span -->
              <!--   class="loader-type" -->
              <!--   v-if="mod.loader !== ModLoader.Unknown" -->
              <!--   :class="{ -->
              <!--     fabric: mod.loader === ModLoader.Fabric, -->
              <!--     forge: mod.loader === ModLoader.Forge, -->
              <!--     quilt: mod.loader === ModLoader.Quilt, -->
              <!--     neoforge: mod.loader === ModLoader.NeoForge, -->
              <!--     liteloader: mod.loader === ModLoader.LiteLoader, -->
              <!--   }" -->
              <!--   >{{ mod.loader.charAt(0).toUpperCase() + mod.loader.slice(1) }}</span -->
              <!-- > -->
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
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import Modrinth from "@/assets/images/modrinth.svg";
import CurseForge from "@/assets/images/curseforge.svg";
import AppIcon from "@/components/AppIcon.vue";
import { computed, ref, watch } from "vue";
import { useGameContentStore } from "@/store/content";
import { ModLoader } from "@conic/content";
import {
  SearchedProjects as ModrinthSearchedProjects,
  searchProjects as searchModrinthProjects,
} from "@conic/modrinth";
import { useInstanceStore } from "@/store/instance";
import {
  ApiResponse as CurseForgeApiResponse,
  Mod as CurseForgeMod,
  searchMods as searchCurseForgeMods,
} from "@conic/curseforge";
import ScrollView from "@/components/ScrollView.vue";

const gameContentStore = useGameContentStore();
const mods = computed(() => gameContentStore.gameContent.mods);

const currentView = ref("local" as "local" | "modrinth" | "curseforge");

const instanceStore = useInstanceStore();
const modrinthSearchResult = ref(null as null | ModrinthSearchedProjects);
const curseForgeSearchResult = ref(null as null | CurseForgeApiResponse<CurseForgeMod[]>);
watch(currentView, async (value) => {
  if (
    !instanceStore.currentInstance.config.runtime.mod_loader_type ||
    !instanceStore.currentInstance.config.runtime.mod_loader_version
  ) {
    return;
  }
  if (value === "modrinth" && modrinthSearchResult.value === null) {
    try {
      modrinthSearchResult.value = await searchModrinthProjects({
        facets: `[["project_type:mod"],["categories:${instanceStore.currentInstance.config.runtime.mod_loader_type.toLowerCase()}"]]`,
      });
    } catch (error) {
      console.error(error);
    }
  }
  if (value === "curseforge" && curseForgeSearchResult.value === null) {
    try {
      curseForgeSearchResult.value = await searchCurseForgeMods({});
    } catch (error) {
      console.error(error);
    }
  }
});
</script>

<style lang="less" scoped>
.content-mods {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;
  .mods-list-wrapper {
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
    .select-source {
      display: flex;
      align-items: center;
      border-radius: 4px;
      overflow: hidden;
      margin-left: auto;
      border: 1px solid var(--ctp-surface1);
      button {
        appearance: none;
        border: none;
        background: none;
        background: var(--ctp-surface0);
        width: 40px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-right: 1px solid var(--ctp-surface1);
      }
      button:last-child {
        border-right: none;
      }
      button.active {
        background: var(--ctp-lavender);
      }
    }
  }
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
