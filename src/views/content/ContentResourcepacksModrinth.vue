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
          v-for="(pack, index) in modrinthSearchResult.hits"
          class="content"
          :key="index"
          @click="useShowContentDetails().value.modrinth.resourcepack = pack.project_id">
          <img
            v-if="pack.icon_url"
            :src="pack.icon_url"
            alt="pack icon"
            width="72px"
            height="100%" />
          <img
            v-else
            src="@/assets/images/Unknown_server.webp"
            alt="pack icon"
            width="72px"
            height="100%" />
          <div class="content-info">
            <p class="name">
              <span>{{ pack.title }}</span>
            </p>
            <p class="authors">by {{ pack.author }}</p>
            <p class="mod-description">
              {{ modrinthTranslations.get(pack.project_id) ?? pack.description }}
            </p>
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
                  openUrl(`https://modrinth.com/${pack.project_type}/${pack.slug}`)
                "></AppIcon>
            </button>
          </div>
        </div>
      </div>
      <div class="search-status" v-else>
        <ContentNotFound description="尝试调整关键词或筛选条件后再次搜索" show />
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
import { computed, onMounted, ref } from "vue";
import {
  SearchedProjects as ModrinthSearchedProjects,
  SearchParameters as ModrinthSearchParameters,
  searchProjects as searchModrinthProjects,
} from "@conic/modrinth";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { useShowContentDetails } from "./useContent";
import { useSearchPagination } from "./useSearchPagination";
import BaseLoading from "@/components/BaseLoading.vue";
import AppIcon from "@/components/AppIcon.vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import ContentNotFound from "./ContentNotFound.vue";

const { modrinthCache: modrinthTranslations, translateModrinthDescriptions } =
  useDescriptionTranslation();

const CATEGORIES = [
  "faithful",
  "16x",
  "32x",
  "64x",
  "128x",
  "256x",
  "photo-realistic",
  "semi-realistic",
  "simple",
  "modern",
  "theme-based",
  "classic",
  "dark",
  "medieval",
  "anime",
  "cartoon",
  "pixel-art",
  "vanilla-plus",
  "utility",
  "other",
];
const CATEGORY_NAMES: Record<string, string> = {
  faithful: "忠实原版",
  "16x": "16x",
  "32x": "32x",
  "64x": "64x",
  "128x": "128x",
  "256x": "256x",
  "photo-realistic": "照片写实",
  "semi-realistic": "半写实",
  simple: "简约",
  modern: "现代",
  "theme-based": "主题风格",
  classic: "经典",
  dark: "暗色",
  medieval: "中世纪",
  anime: "动漫",
  cartoon: "卡通",
  "pixel-art": "像素画",
  "vanilla-plus": "原版加强",
  utility: "实用",
  other: "其他",
};
type FilterOption = string;
type ModsFilter = {
  key: "version" | "category";
  label: string;
  options: FilterOption[];
  isSelected: (option: FilterOption) => boolean;
  toggle: (option: FilterOption) => void;
  display: (option: FilterOption) => string;
};

const selectedVersions = ref<string[]>([]);
const selectedCategories = ref<string[]>([]);

const modrinthLoading = ref(false);
const modrinthSearchResult = ref(null as null | ModrinthSearchedProjects);
const modrinthCache = new Map<string, ModrinthSearchedProjects>();

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
  () => totalPages.value,
  selectedVersions,
);

function buildModrinthFacets(): string {
  const facets: string[][] = [["project_type:resourcepack"]];
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

const modrinthFilters = computed<ModsFilter[]>(() => [
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

let modrinthInitializedFor: string | null = null;

async function ensureModrinthInitialized() {
  if (versionOptions.value.length === 0) {
    await loadVersionOptions();
  }
  const key = searchInitKey();
  if (modrinthInitializedFor === key) return;
  modrinthInitializedFor = key;
  const runtime = instanceStore.currentInstance?.config.runtime;
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
