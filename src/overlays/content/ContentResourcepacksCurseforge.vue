<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <ContentSearchPanel
      v-model="searchQuery"
      :filters="curseForgeFilters"
      :placeholder="t('overlays.content.common.searchResourcePacks')"
      :version-page="versionPage"
      :version-page-count="versionPageCount"
      :version-track-style="versionTrackStyle"
      :set-version-track-ref="setVersionTrackRef"
      @search="applySearchFilters()"
      @filter-change="onFilterChipClick"
      @version-page-prev="versionPagePrev()"
      @version-page-next="versionPageNext()" />

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
        <ContentNotFound :description="t('overlays.content.common.notFoundDesc')" show />
      </div>
    </template>

    <ContentPagination
      :total-pages="curseForgeTotalPages"
      :current-page="currentPage"
      :pages="paginationPages"
      @page-change="goToPage" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  ApiResponse as CurseForgeApiResponse,
  Mod as CurseForgeMod,
  SearchModsParams as CurseForgeSearchParams,
  searchMods as searchCurseForgeMods,
} from "@conic/curseforge";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { useShowContentDetails } from "./useContent";
import { useSearchPagination } from "./useSearchPagination";
import type { ContentFilterItem } from "./ContentSearchPanel.vue";
import ContentSearchPanel from "./ContentSearchPanel.vue";
import ContentPagination from "./ContentPagination.vue";
import BaseLoading from "@/components/BaseLoading.vue";
import AppIcon from "@/components/AppIcon.vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import ContentNotFound from "./ContentNotFound.vue";

const { t } = useI18n();

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
type ModsFilter = ContentFilterItem & {
  key: "version" | "category";
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
} = useSearchPagination(() => curseForgeTotalPages.value, curseForgeSelectedVersions);

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
    label: t("overlays.content.common.version"),
    options: versionOptions.value,
    isSelected: (option) => curseForgeSelectedVersions.value.includes(option as string),
    display: (option) => option as string,
  },
  {
    key: "category",
    label: t("overlays.content.common.category"),
    options: CURSEFORGE_CATEGORIES,
    isSelected: (option) =>
      curseForgeSelectedCategories.value.includes((option as CategoryOption).id),
    display: (option) =>
      t(`overlays.content.resourcepacks.curseforge.${(option as CategoryOption).slug}`) ??
      (option as CategoryOption).slug,
  },
]);

function onFilterChipClick(filter: ContentFilterItem, option: unknown) {
  if (filter.key === "version") {
    toggleFilterOption(curseForgeSelectedVersions.value, option as string);
  } else if (filter.key === "category") {
    toggleFilterOption(curseForgeSelectedCategories.value, (option as CategoryOption).id);
  }
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
