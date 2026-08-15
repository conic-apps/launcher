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
          :placeholder="'搜索资源包...'"
          v-model="searchQuery"
          @keyup.enter="applySearchFilters()" />
        <button class="search-button" @click="applySearchFilters()">
          <AppIcon name="search" :size="16"></AppIcon>
        </button>
      </div>
      <div class="filter-bar">
        <div class="filter-row" v-for="filter in curseForgeFilters" :key="filter.key">
          <span class="filter-label">{{ filter.label }}</span>
          <div class="filter-chips" :class="{ paged: filter.key === 'version' }">
            <template v-if="filter.key === 'version'">
              <button class="chip-pager" :disabled="versionPage <= 0" @click="versionPagePrev()">
                ‹
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
                ›
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

    <p class="result-count" v-if="curseForgeSearchResult">
      {{ `共 ${curseForgeSearchResult.pagination?.totalCount ?? 0} 个资源包` }}
    </p>

    <div class="search-status" v-if="curseForgeSearchResult === null || curseForgeLoading">
      <span>{{ "正在搜索..." }}</span>
    </div>
    <template v-else>
      <div class="mods-list" v-if="curseForgeSearchResult.data.length > 0">
        <div v-for="(mod, index) in curseForgeSearchResult.data" class="content" :key="index">
          <img v-if="mod.logo.url" :src="mod.logo.url" alt="pack icon" width="72px" height="100%" />
          <img
            v-else
            src="@/assets/images/Unknown_server.webp"
            alt="pack icon"
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
            <span class="version" v-if="mod.latestFilesIndexes && mod.latestFilesIndexes[0]">{{
              mod.latestFilesIndexes[0].gameVersion
            }}</span>
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
      <div class="search-status" v-else>
        <span>{{ "没有找到相关资源包" }}</span>
      </div>
    </template>

    <div class="pagination" v-if="curseForgeTotalPages > 1">
      <button class="page-nav" :disabled="curseForgePage === 1" @click="goToPage(1)">«</button>
      <button
        class="page-nav"
        :disabled="curseForgePage === 1"
        @click="goToPage(curseForgePage - 1)">
        ‹
      </button>
      <template v-for="(page, index) in paginationPages" :key="index">
        <button
          v-if="page !== '…'"
          class="page-number"
          :class="{ active: page === curseForgePage }"
          @click="goToPage(page)">
          {{ page }}
        </button>
        <span v-else class="page-ellipsis">…</span>
      </template>
      <button
        class="page-nav"
        :disabled="curseForgePage === curseForgeTotalPages"
        @click="goToPage(curseForgePage + 1)">
        ›
      </button>
      <button
        class="page-nav"
        :disabled="curseForgePage === curseForgeTotalPages"
        @click="goToPage(curseForgeTotalPages)">
        »
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useInstanceStore } from "@/store/instance";
import { getMinecrafVersionManifest } from "@conic/install";
import {
  ApiResponse as CurseForgeApiResponse,
  Mod as CurseForgeMod,
  SearchModsParams as CurseForgeSearchParams,
  searchMods as searchCurseForgeMods,
} from "@conic/curseforge";

const instanceStore = useInstanceStore();

const PAGE_SIZE = 20;
const VERSIONS_PER_PAGE = 6;
type CategoryOption = { id: number; slug: string };
const CURSEFORGE_CATEGORIES: CategoryOption[] = [
  { id: 4252, slug: "crafted" },
  { id: 4255, slug: "photo-realistic" },
  { id: 4259, slug: "semi-realistic" },
  { id: 4256, slug: "simple" },
  { id: 4258, slug: "traditional" },
  { id: 4253, slug: "animated" },
  { id: 4254, slug: "modern" },
  { id: 4257, slug: "themed" },
  { id: 4261, slug: "mod-support" },
  { id: 4264, slug: "rpg" },
  { id: 4266, slug: "gameplay" },
  { id: 4268, slug: "gui" },
  { id: 4269, slug: "sound" },
  { id: 4270, slug: "environment" },
  { id: 4271, slug: "world-gen" },
  { id: 4273, slug: "blocks" },
  { id: 4274, slug: "items" },
  { id: 4275, slug: "mobs" },
  { id: 4276, slug: "weather" },
];
const CURSEFORGE_CATEGORY_NAMES: Record<string, string> = {
  crafted: "手工制作",
  "photo-realistic": "照片写实",
  "semi-realistic": "半写实",
  simple: "简约",
  traditional: "传统",
  animated: "动态",
  modern: "现代",
  themed: "主题风格",
  "mod-support": "模组支持",
  rpg: "RPG",
  gameplay: "游戏玩法",
  gui: "界面",
  sound: "声音",
  environment: "环境",
  "world-gen": "世界生成",
  blocks: "方块",
  items: "物品",
  mobs: "生物",
  weather: "天气",
};
type FilterOption = string | CategoryOption;
type ModsFilter = {
  key: "version" | "category";
  label: string;
  options: FilterOption[];
  isSelected: (option: FilterOption) => boolean;
  toggle: (option: FilterOption) => void;
  display: (option: FilterOption) => string;
};

const searchQuery = ref("");
const curseForgeSelectedVersions = ref<string[]>([]);
const curseForgeSelectedCategories = ref<number[]>([]);
const versionOptions = ref<string[]>([]);
const versionPage = ref(0);
const versionOffset = ref(0);
const versionPageAnimated = ref(false);
let versionTrackElement: HTMLElement | null = null;
function setVersionTrackRef(el: unknown) {
  versionTrackElement = el instanceof HTMLElement ? el : null;
}

const curseForgePage = ref(1);
const curseForgeLoading = ref(false);
const curseForgeSearchResult = ref(null as null | CurseForgeApiResponse<CurseForgeMod[]>);
const curseForgeCache = new Map<string, CurseForgeApiResponse<CurseForgeMod[]>>();

function toggleFilterOption<T>(list: T[], value: T) {
  const index = list.indexOf(value);
  if (index >= 0) {
    list.splice(index, 1);
  } else {
    list.push(value);
  }
}

function buildCurseForgeParams(): CurseForgeSearchParams {
  const params: CurseForgeSearchParams = {
    classId: 12,
    searchFilter: searchQuery.value.trim() || undefined,
    gameVersions:
      curseForgeSelectedVersions.value.length > 0
        ? JSON.stringify(curseForgeSelectedVersions.value.slice(0, 4))
        : undefined,
    categoryIds:
      curseForgeSelectedCategories.value.length > 0
        ? JSON.stringify(curseForgeSelectedCategories.value)
        : undefined,
    index: (curseForgePage.value - 1) * PAGE_SIZE,
    pageSize: PAGE_SIZE,
  };
  return params;
}

let curseForgeSearchToken = 0;

async function runCurseForgeSearch() {
  const token = ++curseForgeSearchToken;
  const params = buildCurseForgeParams();
  const cacheKey = JSON.stringify(params);
  const cached = curseForgeCache.get(cacheKey);
  if (cached) {
    if (token === curseForgeSearchToken) curseForgeSearchResult.value = cached;
    return;
  }
  curseForgeLoading.value = true;
  try {
    const result = await searchCurseForgeMods(params);
    if (token !== curseForgeSearchToken) return;
    curseForgeCache.set(cacheKey, result);
    curseForgeSearchResult.value = result;
  } catch (error) {
    console.error(error);
  } finally {
    if (token === curseForgeSearchToken) curseForgeLoading.value = false;
  }
}

function applySearchFilters() {
  curseForgePage.value = 1;
  void runCurseForgeSearch();
}

function goToPage(page: number) {
  if (page < 1 || page > curseForgeTotalPages.value) return;
  curseForgePage.value = page;
  void runCurseForgeSearch();
}

const curseForgeTotalPages = computed(() => {
  const totalCount = curseForgeSearchResult.value?.pagination?.totalCount;
  if (!totalCount) return 0;
  return Math.max(1, Math.ceil(totalCount / PAGE_SIZE));
});

const paginationPages = computed(() => {
  const total = curseForgeTotalPages.value;
  const current = curseForgePage.value;
  const pages: (number | "…")[] = [];
  if (total <= 7) {
    for (let page = 1; page <= total; page++) pages.push(page);
    return pages;
  }
  pages.push(1);
  if (current > 3) pages.push("…");
  for (let page = Math.max(2, current - 1); page <= Math.min(total - 1, current + 1); page++) {
    pages.push(page);
  }
  if (current < total - 2) pages.push("…");
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

const curseForgeFilters = computed<ModsFilter[]>(() => [
  {
    key: "version",
    label: "版本",
    options: versionOptions.value,
    isSelected: (option) => curseForgeSelectedVersions.value.includes(option as string),
    toggle: (option) => toggleFilterOption(curseForgeSelectedVersions.value, option as string),
    display: (option) => option as string,
  },
  {
    key: "category",
    label: "分类",
    options: CURSEFORGE_CATEGORIES,
    isSelected: (option) =>
      curseForgeSelectedCategories.value.includes((option as CategoryOption).id),
    toggle: (option) =>
      toggleFilterOption(curseForgeSelectedCategories.value, (option as CategoryOption).id),
    display: (option) =>
      CURSEFORGE_CATEGORY_NAMES[(option as CategoryOption).slug] ?? (option as CategoryOption).slug,
  },
]);

function onFilterChipClick(filter: ModsFilter, option: FilterOption) {
  filter.toggle(option);
  curseForgePage.value = 1;
  void runCurseForgeSearch();
}

async function loadVersionOptions() {
  try {
    const manifest = await getMinecrafVersionManifest();
    const options = manifest.versions
      .filter((version) => version.type === "release")
      .sort((a, b) => new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime())
      .map((version) => version.id);
    const minecraft = instanceStore.currentInstance.config.runtime.minecraft;
    if (minecraft && !options.includes(minecraft)) {
      options.push(minecraft);
    }
    versionOptions.value = options;
  } catch (error) {
    console.error(error);
  }
}

function syncVersionPageToSelection() {
  const current = curseForgeSelectedVersions.value[0];
  if (!current) {
    versionPage.value = 0;
    return;
  }
  const index = versionOptions.value.indexOf(current);
  versionPage.value = index >= 0 ? Math.floor(index / VERSIONS_PER_PAGE) : 0;
}

function searchInitKey(): string {
  const runtime = instanceStore.currentInstance.config.runtime;
  return `${runtime.mod_loader_type ?? ""}|${runtime.minecraft}`;
}

let curseForgeInitializedFor: string | null = null;

async function ensureCurseForgeInitialized() {
  if (versionOptions.value.length === 0) {
    await loadVersionOptions();
  }
  const key = searchInitKey();
  if (curseForgeInitializedFor === key) return;
  curseForgeInitializedFor = key;
  const runtime = instanceStore.currentInstance.config.runtime;
  curseForgeSelectedVersions.value = runtime.minecraft ? [runtime.minecraft] : [];
  searchQuery.value = "";
  syncVersionPageToSelection();
  curseForgeSelectedCategories.value = [];
  curseForgePage.value = 1;
  curseForgeSearchResult.value = null;
}

onMounted(async () => {
  await ensureCurseForgeInitialized();
  void updateVersionOffset();
  await runCurseForgeSearch();
});
</script>

<style lang="less" scoped>
.mods-list-wrapper {
  padding: 16px 32px 32px 32px;
}
.search-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 16px;
  padding: 12px;
  border: 1px solid var(--ctp-surface1);
  border-radius: 10px;
  background: rgba(var(--ctp-mantle-rgb), 0.92);
  backdrop-filter: blur(4px);

  .search-bar {
    display: flex;
    gap: 8px;

    .search-input {
      flex: 1;
      height: 36px;
      padding: 0 12px;
      border: 1px solid var(--ctp-surface1);
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
    align-items: flex-start;
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
    height: 26px;
    padding: 0 12px;
    border: 1px solid var(--ctp-surface1);
    border-radius: 999px;
    background: var(--ctp-surface0);
    color: var(--ctp-text);
    font-size: 12px;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease;

    &:hover {
      background: var(--ctp-surface1);
    }

    &:active {
      background: var(--ctp-surface2);
    }

    &.selected {
      border-color: var(--ctp-lavender);
      background: var(--ctp-lavender);
      color: var(--ctp-text-inverse);
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
}
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 20px 0 8px;

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

    &:hover:not(:disabled) {
      background: var(--ctp-surface1);
    }

    &:active:not(:disabled) {
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
    min-width: 20px;
    font-size: 12px;
    color: var(--ctp-subtext0);
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
      span.version {
        font-size: 9px;
        padding: 2px 6px;
        margin-right: 4px;
        border-radius: 100px;
        font-weight: 500;
        color: var(--ctp-text);
        border: 1px solid var(--ctp-sky);
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
