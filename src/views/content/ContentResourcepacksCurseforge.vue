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
          autocapitalize="off"
          autocomplete="off"
          autocorrect="off"
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

    <div class="search-status" v-if="curseForgeSearchResult === null || curseForgeLoading">
      <div class="loading">
        <BaseLoading :size="32" :gap="8" :strokeWidth="4"></BaseLoading>
      </div>
    </div>
    <template v-else>
      <div class="mods-list" v-if="curseForgeSearchResult.data.length > 0">
        <div
          v-for="(mod, index) in curseForgeSearchResult.data"
          class="content"
          :key="index"
          @click="useShowContentDetails().value.curseforge.resourcepack = mod.id">
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
            <p class="mod-description">{{ curseforgeTranslations.get(mod.id) ?? mod.summary }}</p>
            <span class="version" v-if="mod.latestFilesIndexes && mod.latestFilesIndexes[0]">{{
              mod.latestFilesIndexes[0].gameVersion
            }}</span>
          </div>
          <div class="actions" @click.stop>
            <button class="heart">
              <AppIcon name="heart-outline" :size="14"></AppIcon>
            </button>
            <button class="link">
              <AppIcon name="link" :size="14" @click.stop="openUrl(mod.links.websiteUrl)"></AppIcon>
            </button>
          </div>
        </div>
      </div>
      <div class="search-status" v-else>
        <ContentNotFound description="尝试调整关键词或筛选条件后再次搜索" show />
      </div>
    </template>

    <div class="pagination" v-if="curseForgeTotalPages > 1">
      <button
        class="page-nav"
        :disabled="currentPage === 1"
        @click="goToPage(currentPage - 1)">
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
        :disabled="currentPage === curseForgeTotalPages"
        @click="goToPage(currentPage + 1)">
        <AppIcon name="chevron-forward" :size="12"></AppIcon>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  ApiResponse as CurseForgeApiResponse,
  Mod as CurseForgeMod,
  SearchModsParams as CurseForgeSearchParams,
  searchMods as searchCurseForgeMods,
} from "@conic/curseforge";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { useShowContentDetails } from "./useContent";
import { useSearchPagination } from "./useSearchPagination";
import BaseLoading from "@/components/BaseLoading.vue";
import AppIcon from "@/components/AppIcon.vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import ContentNotFound from "./ContentNotFound.vue";

const { curseforgeCache: curseforgeTranslations, translateCurseforgeSummaries } =
  useDescriptionTranslation();

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

const curseForgeSelectedVersions = ref<string[]>([]);
const curseForgeSelectedCategories = ref<number[]>([]);

const curseForgeLoading = ref(false);
const curseForgeSearchResult = ref(null as null | CurseForgeApiResponse<CurseForgeMod[]>);
const curseForgeCache = new Map<string, CurseForgeApiResponse<CurseForgeMod[]>>();

const {
  PAGE_SIZE,
  searchQuery,
  currentPage,
  versionOptions,
  versionPage,
  setVersionTrackRef,
  toggleFilterOption,
  paginationPages,
  versionPageCount,
  versionTrackStyle,
  updateVersionOffset,
  versionPagePrev,
  versionPageNext,
  syncVersionPageToSelection,
  searchInitKey,
  loadVersionOptions,
  instanceStore,
} = useSearchPagination(
  () => curseForgeTotalPages.value,
  curseForgeSelectedVersions,
);

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
    index: (currentPage.value - 1) * PAGE_SIZE,
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
    if (token === curseForgeSearchToken) {
      curseForgeSearchResult.value = cached;
      void translateCurseforgeSummaries(cached.data.map((mod) => mod.id));
    }
    return;
  }
  curseForgeLoading.value = true;
  try {
    const result = await searchCurseForgeMods(params);
    if (token !== curseForgeSearchToken) return;
    curseForgeCache.set(cacheKey, result);
    curseForgeSearchResult.value = result;
    void translateCurseforgeSummaries(result.data.map((mod) => mod.id));
  } catch (error) {
    console.error(error);
  } finally {
    if (token === curseForgeSearchToken) curseForgeLoading.value = false;
  }
}

function applySearchFilters() {
  currentPage.value = 1;
  void runCurseForgeSearch();
}

function goToPage(page: number) {
  if (page < 1 || page > curseForgeTotalPages.value) return;
  currentPage.value = page;
  void runCurseForgeSearch();
}

const curseForgeTotalPages = computed(() => {
  const totalCount = curseForgeSearchResult.value?.pagination?.totalCount;
  if (!totalCount) return 0;
  return Math.max(1, Math.ceil(totalCount / PAGE_SIZE));
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
  currentPage.value = 1;
  void runCurseForgeSearch();
}

let curseForgeInitializedFor: string | null = null;

async function ensureCurseForgeInitialized() {
  if (versionOptions.value.length === 0) {
    await loadVersionOptions();
  }
  const key = searchInitKey();
  if (curseForgeInitializedFor === key) return;
  curseForgeInitializedFor = key;
  const runtime = instanceStore.currentInstance?.config.runtime;
  curseForgeSelectedVersions.value = runtime?.minecraft ? [runtime.minecraft] : [];
  searchQuery.value = "";
  syncVersionPageToSelection();
  curseForgeSelectedCategories.value = [];
  currentPage.value = 1;
  curseForgeSearchResult.value = null;
}

onMounted(async () => {
  await ensureCurseForgeInitialized();
  void updateVersionOffset();
  await runCurseForgeSearch();
});
</script>

<style lang="less" scoped>
@import "./styles/search-panel.less";
@import "./styles/pagination.less";
@import "./styles/content-card.less";

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

.mods-list {
  &:extend(.content-card-grid all);
}
</style>
