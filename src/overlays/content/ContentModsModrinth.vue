<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods-list-wrapper">
    <ContentSearchPanel
      v-model="searchQuery"
      :filters="modrinthFilters"
      :placeholder="t('overlays.content.common.searchMods')"
      :version-page="versionPage"
      :version-page-count="versionPageCount"
      :version-track-style="versionTrackStyle"
      :set-version-track-ref="setVersionTrackRef"
      @search="applySearchFilters()"
      @filter-change="onFilterChipClick"
      @version-page-prev="versionPagePrev()"
      @version-page-next="versionPageNext()" />

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
            <button
              class="heart"
              :class="{ 'heart-favorited': isFavorited('modrinth', 'mod', mod.project_id) }"
              @click.stop="toggleFavorite('modrinth', 'mod', mod.project_id)">
              <AppIcon
                :name="isFavorited('modrinth', 'mod', mod.project_id) ? 'heart' : 'heart-outline'"
                :size="14"></AppIcon>
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
        <ContentNotFound :description="t('overlays.content.common.notFoundDesc')" show />
      </div>
    </template>

    <ContentPagination
      :total-pages="totalPages"
      :current-page="currentPage"
      :pages="paginationPages"
      @page-change="goToPage" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  SearchedProjects as ModrinthSearchedProjects,
  SearchParameters as ModrinthSearchParameters,
  searchProjects as searchModrinthProjects,
} from "@conic/modrinth";
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
import { useFavorites } from "./useFavorites";

const { t } = useI18n();
const { loadFavorites, isFavorited, toggleFavorite } = useFavorites();

const { modrinthCache: modrinthTranslations, translateModrinthDescriptions } =
  useDescriptionTranslation();

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
  "favorites",
];
type ModsFilter = ContentFilterItem & {
  key: "loader" | "version" | "category";
};

const selectedLoaders = ref<string[]>([]);
const selectedVersions = ref<string[]>([]);
const selectedCategories = ref<string[]>([]);
const showFavoritesOnly = ref(false);

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
} = useSearchPagination(() => totalPages.value, selectedVersions);

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

function filterByFavorites(result: ModrinthSearchedProjects): ModrinthSearchedProjects {
  if (!showFavoritesOnly.value) return result;
  return {
    ...result,
    hits: result.hits.filter((hit) => isFavorited("modrinth", "mod", hit.project_id)),
  };
}

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
      modrinthSearchResult.value = filterByFavorites(cached);
      void translateModrinthDescriptions(cached.hits.map((hit) => hit.project_id));
    }
    return;
  }
  modrinthLoading.value = true;
  try {
    const result = await searchModrinthProjects(params);
    if (token !== modrinthSearchToken) return;
    modrinthCache.set(cacheKey, result);
    modrinthSearchResult.value = filterByFavorites(result);
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
    key: "loader",
    label: t("overlays.content.common.loader"),
    options: LOADERS,
    isSelected: (option) => selectedLoaders.value.includes(option as string),
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
    label: t("overlays.content.common.version"),
    options: versionOptions.value,
    isSelected: (option) => selectedVersions.value.includes(option as string),
    display: (option) => option as string,
    chipClass: () => ({ "minecraft-version": true }),
  },
  {
    key: "category",
    label: t("overlays.content.common.category"),
    options: CATEGORIES,
    isSelected: (option) =>
      option === "favorites"
        ? showFavoritesOnly.value
        : selectedCategories.value.includes(option as string),
    display: (option) =>
      option === "favorites"
        ? t("overlays.content.common.favorites")
        : (t(`overlays.content.mods.modrinth.${option as string}`) ?? option),
    chipClass: (option) => ({ favorites: option === "favorites" }),
  },
]);

function onFilterChipClick(filter: ContentFilterItem, option: unknown) {
  if (filter.key === "loader") {
    toggleFilterOption(selectedLoaders.value, option as string);
  } else if (filter.key === "version") {
    toggleFilterOption(selectedVersions.value, option as string);
  } else if (filter.key === "category") {
    if (option === "favorites") {
      showFavoritesOnly.value = !showFavoritesOnly.value;
    } else {
      showFavoritesOnly.value = false;
      toggleFilterOption(selectedCategories.value, option as string);
    }
  }
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
  selectedLoaders.value = runtime?.mod_loader_type ? [runtime.mod_loader_type.toLowerCase()] : [];
  selectedVersions.value = runtime?.minecraft ? [runtime.minecraft] : [];
  searchQuery.value = "";
  syncVersionPageToSelection();
  selectedCategories.value = [];
  showFavoritesOnly.value = false;
  currentPage.value = 1;
  modrinthSearchResult.value = null;
}

onMounted(async () => {
  await loadFavorites();
  await ensureModrinthInitialized();
  void updateVersionOffset();
  await runModrinthSearch();
});

function openDetails(projectId: string) {
  useShowContentDetails().value.modrinth.mod = projectId;
}
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

.heart-favorited {
  color: var(--ctp-red) !important;
  opacity: 1 !important;
}
</style>
