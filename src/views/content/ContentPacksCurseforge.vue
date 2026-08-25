<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <ContentSearchPanel
      v-model="searchQuery"
      :filters="curseForgeFilters"
      :placeholder="t('content.common.searchPacks')"
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
          @click="useShowContentDetails().value.curseforge.pack = mod.id">
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
        <ContentNotFound :description="t('content.common.notFoundDesc')" show />
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
  ModLoaderType as CurseForgeModLoaderType,
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

const LOADERS = ["fabric", "forge", "neoforge", "quilt"];
const LOADER_NAMES: Record<string, string> = {
  fabric: "Fabric",
  forge: "Forge",
  neoforge: "NeoForge",
  quilt: "Quilt",
};
const CURSEFORGE_LOADER_ENUMS: Record<string, CurseForgeModLoaderType> = {
  forge: CurseForgeModLoaderType.Forge,
  fabric: CurseForgeModLoaderType.Fabric,
  quilt: CurseForgeModLoaderType.Quilt,
  neoforge: CurseForgeModLoaderType.NeoForge,
};
type CategoryOption = { id: number; slug: string };
const CURSEFORGE_CATEGORIES: CategoryOption[] = [
  { id: 4472, slug: "tech" },
  { id: 4473, slug: "magic" },
  { id: 4474, slug: "sci-fi" },
  { id: 4475, slug: "adventure-and-rpg" },
  { id: 4476, slug: "exploration" },
  { id: 4477, slug: "mini-game" },
  { id: 4478, slug: "quests" },
  { id: 4479, slug: "hardcore" },
  { id: 4480, slug: "map-based" },
  { id: 4481, slug: "small-light" },
  { id: 4482, slug: "extra-large" },
  { id: 4483, slug: "combat" },
  { id: 4484, slug: "multiplayer" },
  { id: 4487, slug: "ftb" },
  { id: 4736, slug: "skyblock" },
  { id: 5128, slug: "vanilla-plus" },
  { id: 7418, slug: "horror" },
  { id: 9243, slug: "expert" },
];
type ModsFilter = ContentFilterItem & {
  key: "loader" | "version" | "category";
};

const curseForgeSelectedLoaders = ref<string[]>([]);
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
  loadVersionOptions,
} = useSearchPagination(() => curseForgeTotalPages.value, curseForgeSelectedVersions);

function buildCurseForgeParams(): CurseForgeSearchParams {
  const params: CurseForgeSearchParams = {
    classId: 4471,
    searchFilter: searchQuery.value.trim() || undefined,
    gameVersions:
      curseForgeSelectedVersions.value.length > 0
        ? JSON.stringify(curseForgeSelectedVersions.value.slice(0, 4))
        : undefined,
    modLoaderTypes:
      curseForgeSelectedLoaders.value.length > 0
        ? JSON.stringify(
            curseForgeSelectedLoaders.value
              .map((loader) => CURSEFORGE_LOADER_ENUMS[loader])
              .slice(0, 5),
          )
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
    key: "loader",
    label: t("content.common.loader"),
    options: LOADERS,
    isSelected: (option) => curseForgeSelectedLoaders.value.includes(option as string),
    display: (option) => LOADER_NAMES[option as string] ?? option,
    chipClass: (option) => ({
      fabric: option === "fabric",
      forge: option === "forge",
      quilt: option === "quilt",
      neoforge: option === "neoforge",
    }),
  },
  {
    key: "version",
    label: t("content.common.version"),
    options: versionOptions.value,
    isSelected: (option) => curseForgeSelectedVersions.value.includes(option as string),
    display: (option) => option as string,
    chipClass: () => ({ "minecraft-version": true }),
  },
  {
    key: "category",
    label: t("content.common.category"),
    options: CURSEFORGE_CATEGORIES,
    isSelected: (option) =>
      curseForgeSelectedCategories.value.includes((option as CategoryOption).id),
    display: (option) =>
      t(`content.packs.curseforge.${(option as CategoryOption).slug}`) ??
      (option as CategoryOption).slug,
  },
]);

function onFilterChipClick(filter: ContentFilterItem, option: unknown) {
  if (filter.key === "loader") {
    toggleFilterOption(curseForgeSelectedLoaders.value, option as string);
  } else if (filter.key === "version") {
    toggleFilterOption(curseForgeSelectedVersions.value, option as string);
  } else if (filter.key === "category") {
    toggleFilterOption(curseForgeSelectedCategories.value, (option as CategoryOption).id);
  }
  currentPage.value = 1;
  void runCurseForgeSearch();
}

async function ensureCurseForgeInitialized() {
  if (versionOptions.value.length === 0) {
    await loadVersionOptions();
  }
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
