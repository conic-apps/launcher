<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="command-palette-backdrop" v-if="visible" @click="close()">
    <div class="command-palette" @click.stop>
      <div class="palette-input-row">
        <AppIcon name="search" :size="16"></AppIcon>
        <input
          ref="inputRef"
          type="text"
          v-model="query"
          :placeholder="placeholderText"
          :spellcheck="false"
          autocapitalize="off"
          autocomplete="off"
          autocorrect="off"
          @keydown="onKeyDown" />
      </div>
      <div class="palette-results" ref="resultsRef">
        <ScrollView ref="scrollViewRef">
          <template v-if="renderItems.length > 0">
            <template v-for="(item, index) in renderItems" :key="item.key">
              <p class="section-label" v-if="item.sectionLabel">{{ item.sectionLabel }}</p>
              <button
                type="button"
                class="palette-item"
                :class="{ selected: index === selectedIndex }"
                @mousemove="selectedIndex = index"
                @click="performItem(item)">
                <span class="item-icon">
                  <img v-if="item.imageUrl" :src="item.imageUrl" class="item-image-url" alt="" />
                  <AppIcon v-else-if="item.icon" :name="item.icon" :size="18"></AppIcon>
                  <component v-else-if="item.image" :is="item.image" class="item-image"></component>
                </span>
                <span class="item-main">
                  <span class="item-title-line">
                    <span class="item-title">{{ item.title }}</span>
                    <span class="item-author" v-if="item.author">by {{ item.author }}</span>
                  </span>
                  <span class="item-loaders" v-if="item.loaders && item.loaders.length > 0">
                    <span class="loader-tag" v-for="loader in item.loaders" :key="loader">
                      {{ LOADER_NAMES[loader] ?? loader }}
                    </span>
                  </span>
                </span>
                <span class="item-subtitle" v-if="item.subtitle">{{ item.subtitle }}</span>
                <AppIcon
                  v-if="item.hasChildren"
                  class="item-children"
                  name="chevron-forward"
                  :size="14"></AppIcon>
              </button>
            </template>
          </template>
          <div class="palette-empty" v-else-if="mode.type === 'search-online' && onlineSearching">
            <BaseLoading :size="24" :gap="6" :strokeWidth="4"></BaseLoading>
          </div>
          <div
            class="palette-empty"
            v-else-if="mode.type === 'search-online' && query.trim() === ''">
            {{ t("app.commandPalette.typeToSearch") }}
          </div>
          <div class="palette-empty" v-else-if="mode.type === 'search-online' && onlineError">
            {{ t("app.commandPalette.searchFailed") }}
          </div>
          <div class="palette-empty" v-else>{{ t("app.commandPalette.noResults") }}</div>
        </ScrollView>
      </div>
      <div class="palette-footer">
        <p class="breadcrumb">{{ breadcrumbText }}</p>
        <div class="hints">
          <span class="hint"><kbd>↑</kbd><kbd>↓</kbd></span>
          <span class="hint" v-if="mode.type !== 'root'">
            <kbd>⌫</kbd>{{ t("app.commandPalette.backHint") }}
          </span>
          <span class="hint" v-else><kbd>esc</kbd>{{ t("app.commandPalette.closeHint") }}</span>
          <span class="hint primary" v-if="primaryAction">
            {{ primaryAction }}
            <AppIcon name="corner-down-left" :size="14"></AppIcon>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef, watch } from "vue";
import type { Component } from "vue";
import { useI18n } from "vue-i18n";
import { useDialogStore } from "@/store/dialog";
import { useInstanceStore } from "@/store/instance";
import { useNavigationStore } from "@/store/navigation";
import BaseLoading from "@/components/BaseLoading.vue";
import ScrollView from "@/components/ScrollView.vue";
import { searchProjects as searchModrinthProjects } from "@conic/modrinth";
import { searchMods as searchCurseForgeApi } from "@conic/curseforge";
import { useShowContentDetails } from "@/overlays/content/useContent";
import ModrinthImage from "@/assets/images/modrinth.svg";
import CurseForgeImage from "@/assets/images/curseforge.svg";
import type { Instance } from "@conic/instance";

const props = defineProps<{ visible: boolean }>();
const emit = defineEmits(["close"]);

const { t } = useI18n();
const dialogStore = useDialogStore();
const instanceStore = useInstanceStore();
const navigationStore = useNavigationStore();

const inputRef = useTemplateRef<HTMLInputElement>("inputRef");
const scrollViewRef = useTemplateRef("scrollViewRef");

type OnlineSource = "modrinth" | "curseforge";

type OnlineProjectType = "mod" | "modpack" | "resourcepack" | "shader";

type PaletteMode =
  | { type: "root" }
  | { type: "launch-instance" }
  | { type: "search-online"; source: OnlineSource };

type PaletteItem = {
  key: string;
  title: string;
  author?: string;
  loaders?: string[];
  subtitle?: string;
  icon?: string;
  image?: Component;
  imageUrl?: string;
  hasChildren?: boolean;
  action: string;
  perform: () => void;
};

type PaletteRenderItem = PaletteItem & { sectionLabel?: string };

const LOADER_SLUGS = ["fabric", "forge", "quilt", "neoforge"];

const LOADER_NAMES: Record<string, string> = {
  fabric: "Fabric",
  forge: "Forge",
  quilt: "Quilt",
  neoforge: "NeoForge",
};

const CURSEFORGE_CLASS_TYPES: Record<number, OnlineProjectType> = {
  6: "mod",
  12: "resourcepack",
  4471: "modpack",
};

const mode = ref<PaletteMode>({ type: "root" });
const query = ref("");
const selectedIndex = ref(0);
const onlineItems = ref<PaletteItem[]>([]);
const onlineSearching = ref(false);
const onlineError = ref(false);

let onlineSearchToken = 0;
let onlineSearchTimer: number | null = null;

watch(
  () => props.visible,
  async (visible) => {
    if (!visible) return;
    mode.value = { type: "root" };
    query.value = "";
    onlineItems.value = [];
    onlineSearching.value = false;
    onlineError.value = false;
    await nextTick();
    inputRef.value?.focus();
  },
);

watch([query, mode], () => {
  selectedIndex.value = 0;
  scrollViewRef.value?.scrollTo(0, false);
  scheduleOnlineSearch();
});

watch(onlineItems, () => {
  selectedIndex.value = 0;
  scrollViewRef.value?.scrollTo(0, false);
});

watch(selectedIndex, () => {
  void revealSelected();
});

async function revealSelected(smooth = true) {
  await nextTick();
  const wrapper = scrollViewRef.value?.getWrapper();
  if (!wrapper) return;
  const selected = wrapper.querySelector(".palette-item.selected");
  if (!(selected instanceof HTMLElement)) return;
  const wrapperRect = wrapper.getBoundingClientRect();
  const rect = selected.getBoundingClientRect();
  const top = rect.top - wrapperRect.top + wrapper.scrollTop;
  const bottom = top + rect.height;
  if (top < wrapper.scrollTop) {
    scrollViewRef.value?.scrollTo(top - 8, smooth);
  } else if (bottom > wrapper.scrollTop + wrapper.clientHeight) {
    scrollViewRef.value?.scrollTo(bottom - wrapper.clientHeight + 8, smooth);
  }
}

const normalizedQuery = computed(() => query.value.trim().toLowerCase());

function instanceMatches(instance: Instance): boolean {
  if (normalizedQuery.value === "") return true;
  return [
    instance.config.name,
    instance.config.runtime.minecraft,
    instance.config.runtime.mod_loader_type ?? "vanilla",
  ].some((field) => field.toLowerCase().includes(normalizedQuery.value));
}

const filteredInstances = computed(() =>
  instanceStore.instances
    .filter((instance) => instanceMatches(instance))
    .sort((a, b) => b.last_played - a.last_played),
);

const rootCommands = computed<PaletteItem[]>(() => [
  {
    key: "launch-instance",
    icon: "play",
    title: t("app.commandPalette.launchInstance"),
    hasChildren: true,
    action: t("app.commandPalette.open"),
    perform: () => enterMode({ type: "launch-instance" }),
  },
  {
    key: "create-instance",
    icon: "add",
    title: t("app.commandPalette.createInstance"),
    action: t("app.commandPalette.open"),
    perform: () => {
      close();
      dialogStore.createInstance.visible = true;
    },
  },
  {
    key: "add-account",
    icon: "user-add",
    title: t("app.commandPalette.addAccount"),
    action: t("app.commandPalette.open"),
    perform: () => {
      close();
      dialogStore.accountAdd.visible = true;
    },
  },
  {
    key: "search-modrinth",
    image: ModrinthImage as unknown as Component,
    title: t("app.commandPalette.searchModrinth"),
    hasChildren: true,
    action: t("app.commandPalette.open"),
    perform: () => enterMode({ type: "search-online", source: "modrinth" }),
  },
  {
    key: "search-curseforge",
    image: CurseForgeImage as unknown as Component,
    title: t("app.commandPalette.searchCurseForge"),
    hasChildren: true,
    action: t("app.commandPalette.open"),
    perform: () => enterMode({ type: "search-online", source: "curseforge" }),
  },
]);

const matchedCommands = computed(() => {
  if (normalizedQuery.value === "") return rootCommands.value;
  return rootCommands.value.filter((command) =>
    command.title.toLowerCase().includes(normalizedQuery.value),
  );
});

function toInstanceItem(instance: Instance): PaletteItem {
  const runtime = instance.config.runtime;
  const loader = runtime.mod_loader_type;
  return {
    key: `instance-${instance.id}`,
    icon: "minecraft",
    title: instance.config.name,
    subtitle: loader ? `${loader} · ${runtime.minecraft}` : runtime.minecraft,
    action: t("app.commandPalette.select"),
    perform: () => selectInstance(instance),
  };
}

function withSectionLabel(items: PaletteItem[], label: string): PaletteRenderItem[] {
  return items.map((item, index) => (index === 0 ? { ...item, sectionLabel: label } : { ...item }));
}

const renderItems = computed<PaletteRenderItem[]>(() => {
  if (mode.value.type === "root") {
    if (matchedCommands.value.length > 0) {
      return withSectionLabel(matchedCommands.value, t("app.commandPalette.commandsSection"));
    }
    return withSectionLabel(
      filteredInstances.value.map(toInstanceItem),
      t("app.commandPalette.instancesSection"),
    );
  }
  if (mode.value.type === "launch-instance") {
    return filteredInstances.value.map((instance) => ({
      ...toInstanceItem(instance),
      action: t("app.commandPalette.launch"),
    }));
  }
  return withSectionLabel(onlineItems.value, t("app.commandPalette.resultsSection"));
});

const placeholderText = computed(() => {
  switch (mode.value.type) {
    case "launch-instance":
      return t("app.commandPalette.launchPlaceholder");
    case "search-online":
      return mode.value.source === "modrinth"
        ? t("app.commandPalette.modrinthPlaceholder")
        : t("app.commandPalette.curseforgePlaceholder");
    default:
      return t("app.commandPalette.placeholder");
  }
});

const breadcrumbText = computed(() => {
  switch (mode.value.type) {
    case "launch-instance":
      return t("app.commandPalette.launchInstance");
    case "search-online":
      return mode.value.source === "modrinth"
        ? t("app.commandPalette.searchModrinth")
        : t("app.commandPalette.searchCurseForge");
    default:
      return t("app.commandPalette.commandsSection");
  }
});

const primaryAction = computed(() => renderItems.value[selectedIndex.value]?.action ?? "");

function enterMode(nextMode: PaletteMode) {
  mode.value = nextMode;
  query.value = "";
  void nextTick(() => inputRef.value?.focus());
}

function backToRoot() {
  enterMode({ type: "root" });
}

function close() {
  emit("close");
}

function selectInstance(instance: Instance) {
  const shouldLaunch = mode.value.type === "launch-instance";
  instanceStore.currentInstance = instance;
  close();
  if (shouldLaunch) navigationStore.navigate("launch");
}

function scheduleOnlineSearch() {
  if (mode.value.type !== "search-online") return;
  if (onlineSearchTimer !== null) window.clearTimeout(onlineSearchTimer);
  const keyword = query.value.trim();
  if (keyword === "") {
    onlineItems.value = [];
    onlineSearching.value = false;
    onlineError.value = false;
    return;
  }
  const source = mode.value.source;
  onlineSearching.value = true;
  onlineSearchTimer = window.setTimeout(() => void runOnlineSearch(source, keyword), 250);
}

async function runOnlineSearch(source: OnlineSource, keyword: string) {
  const token = ++onlineSearchToken;
  onlineSearching.value = true;
  onlineError.value = false;
  try {
    const items =
      source === "modrinth"
        ? await searchModrinthMods(keyword)
        : await searchCurseForgeModsList(keyword);
    if (token !== onlineSearchToken) return;
    onlineItems.value = items;
  } catch (error) {
    console.error(error);
    if (token !== onlineSearchToken) return;
    onlineItems.value = [];
    onlineError.value = true;
  } finally {
    if (token === onlineSearchToken) onlineSearching.value = false;
  }
}

function categoryLabel(type: OnlineProjectType): string {
  switch (type) {
    case "modpack":
      return t("app.commandPalette.categoryModpack");
    case "resourcepack":
      return t("app.commandPalette.categoryResourcepack");
    case "shader":
      return t("app.commandPalette.categoryShader");
    default:
      return t("app.commandPalette.categoryMod");
  }
}

async function searchModrinthMods(keyword: string): Promise<PaletteItem[]> {
  const result = await searchModrinthProjects({
    query: keyword,
    offset: 0,
    limit: 20,
  });
  return result.hits.map((hit) => ({
    key: `modrinth-${hit.project_id}`,
    image: ModrinthImage as unknown as Component,
    imageUrl: hit.icon_url,
    title: hit.title ?? hit.slug ?? hit.project_id,
    author: hit.author,
    loaders: (hit.display_categories ?? []).filter((category) => LOADER_SLUGS.includes(category)),
    subtitle: categoryLabel(hit.project_type as OnlineProjectType),
    action: t("app.commandPalette.open"),
    perform: () => {
      close();
      if (navigationStore.currentPage !== "game") navigationStore.navigate("game");
      useShowContentDetails().value.modrinth.mod = hit.project_id;
    },
  }));
}

async function searchCurseForgeModsList(keyword: string): Promise<PaletteItem[]> {
  const response = await searchCurseForgeApi({
    searchFilter: keyword,
    index: 0,
    pageSize: 20,
  });
  return response.data.map((mod) => {
    const projectType = mod.classId != null ? CURSEFORGE_CLASS_TYPES[mod.classId] : undefined;
    return {
      key: `curseforge-${mod.id}`,
      image: CurseForgeImage as unknown as Component,
      imageUrl: mod.logo?.thumbnailUrl || undefined,
      title: mod.name,
      author: mod.authors?.[0]?.name,
      loaders: (mod.categories ?? [])
        .filter((category) => LOADER_SLUGS.includes(category.slug))
        .map((category) => category.slug),
      subtitle: projectType ? categoryLabel(projectType) : undefined,
      action: t("app.commandPalette.open"),
      perform: () => {
        close();
        if (navigationStore.currentPage !== "game") navigationStore.navigate("game");
        useShowContentDetails().value.curseforge.mod = mod.id;
      },
    };
  });
}

function performItem(item: PaletteRenderItem) {
  item.perform();
}

function onKeyDown(event: KeyboardEvent) {
  const count = renderItems.value.length;
  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      if (count > 0) selectedIndex.value = (selectedIndex.value + 1) % count;
      break;
    case "ArrowUp":
      event.preventDefault();
      if (count > 0) selectedIndex.value = (selectedIndex.value - 1 + count) % count;
      break;
    case "Enter": {
      event.preventDefault();
      const item = renderItems.value[selectedIndex.value];
      if (item) performItem(item);
      break;
    }
    case "Escape":
      event.preventDefault();
      if (query.value !== "") {
        query.value = "";
      } else if (mode.value.type !== "root") {
        backToRoot();
      } else {
        close();
      }
      break;
    case "Backspace":
      if (query.value === "" && mode.value.type !== "root") {
        event.preventDefault();
        backToRoot();
      }
      break;
  }
}
</script>

<style lang="less" scoped>
.command-palette-backdrop {
  position: fixed;
  inset: 0;
  z-index: 114515;
  background: #00000042;
}

.command-palette {
  position: absolute;
  top: 4px;
  left: 50%;
  transform: translateX(-50%);
  width: calc(100vw - 500px);
  min-width: 600px;
  display: flex;
  flex-direction: column;
  background: var(--ctp-base);
  border: 1px solid rgba(var(--ctp-surface1-rgb), 0.9);
  border-radius: 12px;
  box-shadow: 0 0 50px 0 #00000071;
  overflow: hidden;
}

.palette-input-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);
  color: rgba(var(--default-text-color), 0.8);

  input {
    flex: 1;
    border: none;
    background: none;
    outline: none;
    font-size: 15px;
    color: rgb(var(--default-text-color));

    &::placeholder {
      color: rgba(var(--default-text-color), 0.5);
    }
  }
}

.palette-results {
  position: relative;
  display: flex;
  flex-direction: column;
  max-height: 320px;
  padding: 6px;

  & :deep(.wrapper) {
    min-height: 0;
    flex: 1 1 auto;
  }
}

.section-label {
  margin: 0;
  padding: 6px 10px 2px;
  font-size: 11px;
  color: var(--ctp-subtext0);
}

.palette-item {
  appearance: none;
  border: none;
  background: none;
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  text-align: left;
  font-size: 13px;
  color: rgb(var(--default-text-color));

  .item-icon {
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(var(--default-text-color), 0.85);

    .item-image {
      width: 18px;
      height: 18px;

      & :deep(path),
      & :deep(g) {
        fill: rgba(var(--ctp-text-rgb), 0.85);
      }
    }

    .item-image-url {
      width: 18px;
      height: 18px;
      border-radius: 4px;
      object-fit: cover;
    }
  }

  .item-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;

    .item-title-line {
      display: flex;
      align-items: baseline;
      gap: 6px;
      min-width: 0;

      .item-title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .item-author {
        flex-shrink: 0;
        font-size: 11px;
        color: var(--ctp-subtext0);
      }
    }

    .item-loaders {
      display: flex;
      align-items: center;
      gap: 4px;

      .loader-tag {
        font-size: 10px;
        line-height: 1;
        padding: 2px 5px;
        border-radius: 4px;
        background: var(--ctp-surface0);
        color: var(--ctp-subtext1);
      }
    }
  }

  .item-subtitle {
    flex-shrink: 0;
    font-size: 12px;
    color: var(--ctp-subtext0);
  }

  .item-children {
    flex-shrink: 0;
    color: var(--ctp-overlay1);
  }

  &.selected {
    background: var(--ctp-surface1);
  }
}

.palette-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px 16px;
  text-align: center;
  font-size: 13px;
  color: var(--ctp-subtext0);
}

.palette-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 12px;
  border-top: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);

  .breadcrumb {
    margin: 0;
    font-size: 12px;
    color: var(--ctp-subtext0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hints {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
    font-size: 12px;
    color: var(--ctp-subtext0);

    .hint {
      display: flex;
      align-items: center;
      gap: 4px;

      kbd {
        font-family: inherit;
        font-size: 11px;
        padding: 1px 5px;
        border-radius: 4px;
        background: var(--ctp-surface0);
      }

      &.primary {
        color: rgb(var(--default-text-color));
      }
    }
  }
}
</style>
