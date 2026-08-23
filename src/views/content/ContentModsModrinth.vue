<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <div class="search-panel">
      <div class="search-bar">
        <input
          class="search-input"
          type="text"
          :placeholder="'搜索模组...'"
          v-model="searchQuery"
          @keyup.enter="applySearchFilters()" />
        <button class="search-button" @click="applySearchFilters()">
          <AppIcon name="search" :size="16"></AppIcon>
        </button>
      </div>
      <div class="filter-bar">
        <div class="filter-row" v-for="filter in modrinthFilters" :key="filter.key">
          <span class="filter-label">{{ filter.label }}</span>
          <div class="filter-chips" :class="{ paged: filter.key === 'version' }">
            <template v-if="filter.key === 'version'">
              <button class="chip-pager" :disabled="versionPage <= 0" @click="versionPagePrev()">
                <AppIcon name="chevron-back" :size="12"></AppIcon>
              </button>
              <div class="filter-chips-track" :ref="setVersionTrackRef">
                <div class="filter-chips-track-inner" :style="versionTrackStyle">
                  <button
                    class="filter-chip"
                    :class="{ selected: filter.isSelected(option) }"
                    v-for="(option, index) in filter.options"
                    :key="`${filter.key}-${index}`"
                    @click="onFilterChipClick(filter, option)">
                    {{ filter.display(option) }}
                  </button>
                </div>
              </div>
              <button
                class="chip-pager"
                :disabled="versionPage >= versionPageCount - 1"
                @click="versionPageNext()">
                <AppIcon name="chevron-forward" :size="12"></AppIcon>
              </button>
            </template>
            <template v-else>
              <button
                class="filter-chip"
                :class="{ selected: filter.isSelected(option) }"
                v-for="(option, index) in filter.options"
                :key="`${filter.key}-${index}`"
                @click="onFilterChipClick(filter, option)">
                {{ filter.display(option) }}
              </button>
            </template>
          </div>
        </div>
      </div>
    </div>

    <div class="search-status" v-if="modrinthSearchResult === null || modrinthLoading">
      <div class="loading">
        <BaseLoading :size="32" :gap="8" :strokeWidth="4"></BaseLoading>
      </div>
    </div>
    <template v-else>
      <div class="mods-list" v-if="modrinthSearchResult.hits.length > 0">
        <div
          v-for="(mod, index) in modrinthSearchResult.hits"
          class="content"
          :key="index"
          @click="openDetails(mod.project_id)">
          <img v-if="mod.icon_url" :src="mod.icon_url" alt="mod icon" width="72px" height="100%" />
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
            <p class="mod-description">
              {{ modrinthTranslations.get(mod.project_id) ?? mod.description }}
            </p>
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
          </div>
          <div class="actions" @click.stop>
            <button class="heart">
              <AppIcon name="heart-outline" :size="14"></AppIcon>
            </button>
            <button class="link">
              <AppIcon
                name="link"
                :size="14"
                @click.stop="
                  openUrl(`https://modrinth.com/${mod.project_type}/${mod.slug}`)
                "></AppIcon>
            </button>
          </div>
        </div>
      </div>
      <div class="search-status" v-else>
        <span>{{ "没有找到相关模组" }}</span>
      </div>
    </template>

    <div class="pagination" v-if="totalPages > 1">
      <button class="page-nav" :disabled="currentPage === 1" @click="goToPage(currentPage - 1)">
        <AppIcon name="chevron-back" :size="12"></AppIcon>
      </button>
      <template v-for="(page, index) in paginationPages" :key="index">
        <button
          v-if="page !== '…'"
          class="page-number"
          :class="{ active: page === currentPage }"
          @click="goToPage(page)">
          {{ page }}
        </button>
        <span v-else class="page-ellipsis">…</span>
      </template>
      <button
        class="page-nav"
        :disabled="currentPage === totalPages"
        @click="goToPage(currentPage + 1)">
        <AppIcon name="chevron-forward" :size="12"></AppIcon>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useInstanceStore } from "@/store/instance";
import { getMinecrafVersionManifest } from "@conic/install";
import {
  SearchedProjects as ModrinthSearchedProjects,
  SearchParameters as ModrinthSearchParameters,
  searchProjects as searchModrinthProjects,
} from "@conic/modrinth";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { useShowContentDetails } from "./useContent";
import BaseLoading from "@/components/BaseLoading.vue";
import AppIcon from "@/components/AppIcon.vue";
import { openUrl } from "@tauri-apps/plugin-opener";

const instanceStore = useInstanceStore();
const { modrinthCache: modrinthTranslations, translateModrinthDescriptions } =
  useDescriptionTranslation();

const PAGE_SIZE = 20;
const VERSIONS_PER_PAGE = 6;
const LOADERS = ["fabric", "forge", "neoforge", "quilt"];
const LOADER_NAMES: Record<string, string> = {
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
  quilt: "Quilt",
};
const CATEGORIES = [
  "adventure",
  "cursed",
  "decoration",
  "economy",
  "equipment",
  "food",
  "game-mechanics",
  "library",
  "magic",
  "management",
  "minigame",
  "mobs",
  "optimization",
  "social",
  "storage",
  "technology",
  "transportation",
  "utility",
  "worldgen",
];
const CATEGORY_NAMES: Record<string, string> = {
  adventure: "冒险",
  cursed: "诡异",
  decoration: "装饰",
  economy: "经济",
  equipment: "装备",
  food: "食物",
  "game-mechanics": "游戏机制",
  library: "库",
  magic: "魔法",
  management: "管理",
  minigame: "小游戏",
  mobs: "生物",
  optimization: "优化",
  social: "社交",
  storage: "存储",
  technology: "科技",
  transportation: "交通",
  utility: "实用",
  worldgen: "世界生成",
};
type FilterOption = string;
type ModsFilter = {
  key: "loader" | "version" | "category";
  label: string;
  options: FilterOption[];
  isSelected: (option: FilterOption) => boolean;
  toggle: (option: FilterOption) => void;
  display: (option: FilterOption) => string;
};

const searchQuery = ref("");
const selectedLoaders = ref<string[]>([]);
const selectedVersions = ref<string[]>([]);
const selectedCategories = ref<string[]>([]);
const versionOptions = ref<string[]>([]);
const versionPage = ref(0);
const versionOffset = ref(0);
const versionPageAnimated = ref(false);
let versionTrackElement: HTMLElement | null = null;
function setVersionTrackRef(el: unknown) {
  versionTrackElement = el instanceof HTMLElement ? el : null;
}

const currentPage = ref(1);
const modrinthLoading = ref(false);
const modrinthSearchResult = ref(null as null | ModrinthSearchedProjects);
const modrinthCache = new Map<string, ModrinthSearchedProjects>();

function toggleFilterOption<T>(list: T[], value: T) {
  const index = list.indexOf(value);
  if (index >= 0) {
    list.splice(index, 1);
  } else {
    list.push(value);
  }
}

function buildModrinthFacets(): string {
  const facets: string[][] = [["project_type:mod"]];
  if (selectedLoaders.value.length > 0) {
    facets.push(selectedLoaders.value.map((loader) => `categories:${loader}`));
  }
  if (selectedVersions.value.length > 0) {
    facets.push(selectedVersions.value.map((version) => `versions:${version}`));
  }
  if (selectedCategories.value.length > 0) {
    facets.push(selectedCategories.value.map((category) => `categories:${category}`));
  }
  return JSON.stringify(facets);
}

let modrinthSearchToken = 0;

async function runModrinthSearch() {
  const token = ++modrinthSearchToken;
  const params: ModrinthSearchParameters = {
    query: searchQuery.value.trim() || undefined,
    facets: buildModrinthFacets(),
    offset: (currentPage.value - 1) * PAGE_SIZE,
    limit: PAGE_SIZE,
  };
  const cacheKey = JSON.stringify(params);
  const cached = modrinthCache.get(cacheKey);
  if (cached) {
    if (token === modrinthSearchToken) {
      modrinthSearchResult.value = cached;
      void translateModrinthDescriptions(cached.hits.map((hit) => hit.project_id));
    }
    return;
  }
  modrinthLoading.value = true;
  try {
    const result = await searchModrinthProjects(params);
    if (token !== modrinthSearchToken) return;
    modrinthCache.set(cacheKey, result);
    modrinthSearchResult.value = result;
    void translateModrinthDescriptions(result.hits.map((hit) => hit.project_id));
  } catch (error) {
    console.error(error);
  } finally {
    if (token === modrinthSearchToken) modrinthLoading.value = false;
  }
}

function applySearchFilters() {
  currentPage.value = 1;
  void runModrinthSearch();
}

function goToPage(page: number) {
  if (page < 1 || page > totalPages.value) return;
  currentPage.value = page;
  void runModrinthSearch();
}

const totalPages = computed(() => {
  if (!modrinthSearchResult.value) return 0;
  return Math.max(1, Math.ceil(modrinthSearchResult.value.total_hits / PAGE_SIZE));
});

const paginationPages = computed(() => {
  const total = totalPages.value;
  const current = currentPage.value;
  const pages: (number | "…")[] = [];
  if (total <= 15) {
    for (let page = 1; page <= total; page++) pages.push(page);
    return pages;
  }
  if (current <= 7) {
    for (let page = 1; page <= 13; page++) pages.push(page);
    pages.push("…");
    pages.push(total);
    return pages;
  }
  if (current >= total - 6) {
    pages.push(1);
    pages.push("…");
    for (let page = total - 12; page <= total; page++) pages.push(page);
    return pages;
  }
  pages.push(1);
  pages.push("…");
  for (let page = current - 5; page <= current + 5; page++) pages.push(page);
  pages.push("…");
  pages.push(total);
  return pages;
});

const versionPageCount = computed(() =>
  Math.max(0, Math.ceil(versionOptions.value.length / VERSIONS_PER_PAGE)),
);

const versionTrackStyle = computed(() => ({
  transform: `translateX(${versionOffset.value}px)`,
  transition: versionPageAnimated.value ? "transform 240ms ease" : "none",
}));

async function updateVersionOffset() {
  await nextTick();
  const track = versionTrackElement;
  if (!track) return;
  const chips = track.querySelectorAll(".filter-chip");
  if (chips.length === 0) {
    versionOffset.value = 0;
    return;
  }
  const index = Math.min(versionPage.value * VERSIONS_PER_PAGE, chips.length - 1);
  const chip = chips[index] as HTMLElement;
  versionOffset.value = -chip.offsetLeft;
  if (!versionPageAnimated.value) {
    requestAnimationFrame(() => {
      versionPageAnimated.value = true;
    });
  }
}

function versionPagePrev() {
  versionPage.value = Math.max(0, versionPage.value - 1);
}

function versionPageNext() {
  versionPage.value = Math.min(versionPageCount.value - 1, versionPage.value + 1);
}

watch(versionPage, () => {
  void updateVersionOffset();
});

const modrinthFilters = computed<ModsFilter[]>(() => [
  {
    key: "loader",
    label: "加载器",
    options: LOADERS,
    isSelected: (option) => selectedLoaders.value.includes(option as string),
    toggle: (option) => toggleFilterOption(selectedLoaders.value, option as string),
    display: (option) => LOADER_NAMES[option as string] ?? option,
  },
  {
    key: "version",
    label: "版本",
    options: versionOptions.value,
    isSelected: (option) => selectedVersions.value.includes(option as string),
    toggle: (option) => toggleFilterOption(selectedVersions.value, option as string),
    display: (option) => option as string,
  },
  {
    key: "category",
    label: "分类",
    options: CATEGORIES,
    isSelected: (option) => selectedCategories.value.includes(option as string),
    toggle: (option) => toggleFilterOption(selectedCategories.value, option as string),
    display: (option) => CATEGORY_NAMES[option as string] ?? option,
  },
]);

function onFilterChipClick(filter: ModsFilter, option: FilterOption) {
  filter.toggle(option);
  currentPage.value = 1;
  void runModrinthSearch();
}

async function loadVersionOptions() {
  try {
    const manifest = await getMinecrafVersionManifest();
    const options = manifest.versions
      .filter((version) => version.type === "release")
      .sort((a, b) => new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime())
      .map((version) => version.id);
    const minecraft = instanceStore.currentInstance?.config.runtime.minecraft;
    if (minecraft && !options.includes(minecraft)) {
      options.push(minecraft);
    }
    versionOptions.value = options;
  } catch (error) {
    console.error(error);
  }
}

function syncVersionPageToSelection() {
  const current = selectedVersions.value[0];
  if (!current) {
    versionPage.value = 0;
    return;
  }
  const index = versionOptions.value.indexOf(current);
  versionPage.value = index >= 0 ? Math.floor(index / VERSIONS_PER_PAGE) : 0;
}

function searchInitKey(): string {
  const runtime = instanceStore.currentInstance?.config.runtime;
  return `${runtime?.mod_loader_type ?? ""}|${runtime?.minecraft ?? ""}`;
}

let modrinthInitializedFor: string | null = null;

async function ensureModrinthInitialized() {
  if (versionOptions.value.length === 0) {
    await loadVersionOptions();
  }
  const key = searchInitKey();
  if (modrinthInitializedFor === key) return;
  modrinthInitializedFor = key;
  const runtime = instanceStore.currentInstance?.config.runtime;
  selectedLoaders.value = runtime?.mod_loader_type ? [runtime.mod_loader_type.toLowerCase()] : [];
  selectedVersions.value = runtime?.minecraft ? [runtime.minecraft] : [];
  searchQuery.value = "";
  syncVersionPageToSelection();
  selectedCategories.value = [];
  currentPage.value = 1;
  modrinthSearchResult.value = null;
}

onMounted(async () => {
  await ensureModrinthInitialized();
  void updateVersionOffset();
  await runModrinthSearch();
});

function openDetails(projectId: string) {
  useShowContentDetails().value.modrinth.mod = projectId;
}
</script>

<style lang="less" scoped>
.search-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 16px;
  padding: 24px;
  background: rgba(var(--ctp-crust-rgb), 0.92);
  backdrop-filter: blur(4px);

  .search-bar {
    display: flex;
    gap: 8px;

    .search-input {
      flex: 1;
      height: 36px;
      padding: 0 12px;
      border: 1px solid var(--ctp-surface1);
      border: none;
      border-radius: 8px;
      background: var(--ctp-surface0);
      color: var(--ctp-text);
      font-size: 13px;
      transition:
        background 120ms ease,
        border-color 150ms ease;

      &::placeholder {
        color: var(--ctp-subtext0);
      }

      &:hover {
        background: var(--ctp-surface1);
      }

      &:focus {
        outline: none;
        border-color: var(--ctp-lavender);
      }
    }

    .search-button {
      flex-shrink: 0;
      width: 36px;
      height: 36px;
      display: flex;
      align-items: center;
      justify-content: center;
      border: none;
      border-radius: 8px;
      background: var(--ctp-surface0);
      color: var(--ctp-text);
      transition: background 120ms ease;

      &:hover {
        background: var(--ctp-surface1);
      }

      &:active {
        background: var(--ctp-lavender);
        color: var(--ctp-text-inverse);
      }
    }
  }

  .filter-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: 10px;

    .filter-label {
      flex-shrink: 0;
      min-width: 52px;
      line-height: 26px;
      font-size: 12px;
      color: var(--ctp-subtext0);
    }

    .filter-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;

      &.paged {
        flex: 1;
        min-width: 0;
        flex-wrap: nowrap;
        align-items: center;
      }
    }

    .filter-chips-track {
      flex: 1;
      min-width: 0;
      overflow: hidden;
    }

    .filter-chips-track-inner {
      display: flex;
      flex-wrap: nowrap;
      gap: 6px;
      width: max-content;
      will-change: transform;

      .filter-chip {
        flex-shrink: 0;
      }
    }
  }

  .chip-pager {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--ctp-surface1);
    border-radius: 999px;
    background: var(--ctp-surface0);
    color: var(--ctp-text);
    font-size: 14px;
    transition:
      background 120ms ease,
      color 120ms ease;

    &:hover:not(:disabled) {
      background: var(--ctp-surface1);
    }

    &:disabled {
      opacity: 0.4;
    }
  }

  .filter-chip {
    height: 20px;
    padding: 0 8px;
    border: 1px solid var(--ctp-surface1);
    border: none;
    border-radius: 999px;
    background: var(--ctp-surface0);
    background: none;
    color: var(--ctp-text);
    font-size: 12px;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease;

    &:hover {
      background: var(--ctp-surface1);
      transition: none;
    }

    &:active {
      background: var(--ctp-surface2);
      transition:
        background 120ms ease,
        border-color 120ms ease,
        color 120ms ease;
    }

    &.selected {
      background: var(--ctp-lavender);
      color: var(--ctp-text-inverse);
      transition:
        background 120ms ease,
        border-color 120ms ease,
        color 120ms ease;
    }
  }
}
.result-count {
  margin-bottom: 12px;
  font-size: 12px;
  color: var(--ctp-subtext0);
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
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 20px 0 32px;

  button {
    min-width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--ctp-surface1);
    border-radius: 6px;
    background: var(--ctp-surface0);
    color: var(--ctp-text);
    font-size: 12px;
    transition:
      background 120ms ease,
      border-color 120ms ease;

    &:hover:not(:disabled),
    &:hover:not(.active) {
      background: var(--ctp-surface1);
    }

    &:active:not(:disabled),
    &:hover:not(.active) {
      background: var(--ctp-surface2);
    }

    &:disabled {
      opacity: 0.4;
    }

    &.active {
      border-color: var(--ctp-lavender);
      background: var(--ctp-lavender);
      color: var(--ctp-text-inverse);
      pointer-events: none;
    }
  }

  .page-ellipsis {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 30px;
    font-size: 12px;
    color: var(--ctp-subtext0);
  }
}
.mods-list {
  padding: 16px 32px 32px 32px;
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
