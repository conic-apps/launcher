<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list">
    <InstancesListToolBar
      ref="toolbar"
      :sortLabel="sortLabel"
      :sortOptions="sortOptions"
      :selectSort="selectSort"
      :groupLabel="groupLabel"
      :groupOptions="groupOptions"
      :selectGroup="selectGroup"
      v-model:sortMode="sortMode"
      v-model:groupMode="groupMode"
      v-model:searchQuery="searchQuery" />
    <InstancesListScrollView
      ref="scrollView"
      :disabled="overlaid"
      :scrollbar-top="SCROLLBAR_TOP"
      :scrollbar-bottom="SCROLLBAR_BOTTOM">
      <div class="gap-top"></div>
      <template v-for="group in groups" :key="group.key">
        <div class="group-card" style="opacity: 0" :data-group="group.key">
          <div
            class="group"
            :class="{ collapsed: isCollapsed(group.key) || collapsingKey === group.key }"
            :data-id="`group-${group.key}`"
            @click="toggleGroup(group.key)">
            <AppIcon name="chevron-forward" :size="14"></AppIcon>
            <p>{{ group.title }}</p>
            <span class="group-count">{{ group.instances.length }}</span>
          </div>
        </div>
        <div class="group-content" :data-group-content="group.key">
          <template v-if="!isCollapsed(group.key)">
            <div
              class="card-container"
              :class="{ current: instance.id === instanceStore.currentInstance?.id }"
              v-for="instance in group.instances"
              style="opacity: 0"
              :key="instance.id">
              <div
                class="instance"
                @click="selectInstance(instance)"
                :data-id="instance.id"
                :data-key="`${group.key}:${instance.id}`">
                <p>{{ instance.config.name }}</p>
                <div class="details">
                  <span
                    :class="`tag ${instance.config.runtime.mod_loader_type.toLowerCase()}`"
                    v-if="instance.config.runtime.mod_loader_type"
                    >{{ instance.config.runtime.mod_loader_type }}</span
                  >
                  <span class="tag vanilla" v-else>Vanilla</span>
                  <span class="last-play"
                    ><span class="label">{{ t("game.instances.lastPlayed") }}</span>
                    <span v-if="instance.last_played">{{
                      formatLastPlayed(instance.last_played, timeFormatter)
                    }}</span>
                    <span v-else>{{ t("game.instances.neverPlayed") }}</span>
                  </span>
                </div>
                <img
                  class="instance-background"
                  v-if="instance.has_background && backgroundImagesSrc[instance.id]"
                  v-show="backgroundImagesShow[instance.id]"
                  :src="backgroundImagesSrc[instance.id]"
                  alt=""
                  @load="backgroundImagesShow[instance.id] = true"
                  @error="backgroundImagesShow[instance.id] = false" />
              </div>
            </div>
          </template>
        </div>
      </template>
      <div class="gap-bottom"></div>
    </InstancesListScrollView>
    <Transition name="instance-list-placeholder-transition">
      <div class="instance-list-placeholder" v-if="noMatchedInstances">
        <AppIcon name="about" :size="64"></AppIcon>
        <p class="title">{{ t("game.instances.noMatch") }}</p>
        <p class="desc">{{ t("game.instances.createNew") }}</p>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { useInstanceStore } from "@/store/instance";
import { formatLastPlayed, getBackgroundPath, Instance, InstanceSort } from "@conic/instance";
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import InstancesListToolBar from "./InstancesListToolBar.vue";
import InstancesListScrollView from "./InstancesListScrollView.vue";
import { useShowContent } from "@/overlays/content/useContent";
import { useInstanceSettings } from "@/overlays/useInstanceSettings";
import gsap from "gsap";
import AppIcon from "@/components/AppIcon.vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const timeFormatter = {
  get justNow() {
    return t("game.time.justNow");
  },
  hoursAgo: (hours: number) => t("game.time.hoursAgo", hours),
  get yesterday() {
    return t("game.time.yesterday");
  },
  monthDay: (month: number, day: number) => t("game.time.monthDay", { month, day }),
  yearMonthDay: (year: number, month: number, day: number) =>
    t("game.time.yearMonthDay", { year, month, day }),
};

const instanceStore = useInstanceStore();
const scrollViewRef = useTemplateRef("scrollView");
const toolbarRef = useTemplateRef("toolbar");

let resolveReady: () => void;

const ready = new Promise<void>((resolve) => {
  resolveReady = resolve;
});

const SCROLLBAR_TOP = "calc(44px + 8px + 112px + 6px)";
const SCROLLBAR_BOTTOM = "calc(56px + 4px)";

async function selectInstance(instance: Instance) {
  instanceStore.currentInstance = instance;
  await nextTick();
  scrollViewRef.value?.scrollTo(instance.id, true);
}

onMounted(async () => {
  window.addEventListener("keydown", onKeyDown);
  await nextTick();
  if (instanceStore.currentInstance?.id) {
    scrollViewRef.value?.scrollTo(instanceStore.currentInstance.id, false);
  }
  requestAnimationFrame(() => {
    scrollViewRef.value?.reflow().then(() => resolveReady());
  });
  Object.values(instanceStore.instances).forEach(async (instance) => {
    backgroundImagesSrc.value[instance.id] = await getBackgroundSrc(instance.id);
  });
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
});

const noMatchedInstances = computed(
  () =>
    instanceStore.instances.length === 0 ||
    (searchQuery.value.trim() !== "" && filteredInstances.value.length === 0),
);

const playIntro = () => {
  const tl = gsap.timeline();
  const listIntro = scrollViewRef.value?.playIntro();
  tl.add(toolbarRef.value?.playIntro() ?? gsap.timeline());
  if (listIntro) tl.add(listIntro, "<+0.1145141919810");
  return tl;
};

defineExpose({ playIntro, ready });

export type SortMode = "name" | "version" | "playtime" | "lastplay";
const SORT_MODES: SortMode[] = ["name", "version", "playtime", "lastplay"];
const sortMode = ref<SortMode>(
  SORT_MODES.includes(localStorage.getItem("instancesSortMode") as SortMode)
    ? (localStorage.getItem("instancesSortMode") as SortMode)
    : "playtime",
);
const sortOptions = computed<{ key: SortMode; label: string }[]>(() => [
  { key: "name", label: t("game.instances.sort.name") },
  { key: "version", label: t("game.instances.sort.version") },
  { key: "playtime", label: t("game.instances.sort.playtime") },
  { key: "lastplay", label: t("game.instances.sort.lastplay") },
]);

const SORT_MODE_TO_SORT: Record<SortMode, InstanceSort> = {
  name: "Name",
  version: "Version",
  playtime: "Playtime",
  lastplay: "LastPlayed",
};

watch(sortMode, (mode) => {
  localStorage.setItem("instancesSortMode", mode);
  instanceStore.setSort(SORT_MODE_TO_SORT[mode]);
});

function selectSort(mode: SortMode) {
  sortMode.value = mode;
}

export type GroupMode = "none" | "loader";
const GROUP_MODES: GroupMode[] = ["none", "loader"];
const groupMode = ref<GroupMode>(
  GROUP_MODES.includes(localStorage.getItem("instancesGroupMode") as GroupMode)
    ? (localStorage.getItem("instancesGroupMode") as GroupMode)
    : "none",
);
const groupOptions = computed<{ key: GroupMode; label: string }[]>(() => [
  { key: "none", label: t("game.instances.group.none") },
  { key: "loader", label: t("game.instances.group.loader") },
]);

watch(groupMode, (mode) => {
  localStorage.setItem("instancesGroupMode", mode);
});

function selectGroup(mode: GroupMode) {
  groupMode.value = mode;
}

type GroupKey = "starred" | "all" | "quilt" | "fabric" | "neoforge" | "forge" | "vanilla";

interface InstanceGroup {
  key: GroupKey;
  title: string;
  instances: Instance[];
}

const LOADER_GROUPS: { key: Exclude<GroupKey, "starred" | "all">; title: string }[] = [
  { key: "quilt", title: "Quilt" },
  { key: "fabric", title: "Fabric" },
  { key: "neoforge", title: "Neoforge" },
  { key: "forge", title: "Forge" },
  { key: "vanilla", title: "Vanilla" },
];

const SAVED_GROUPS_EXPANDED_KEY = "instancesGroupExpanded";

function loadExpanded(): Partial<Record<GroupKey, boolean>> {
  try {
    const raw = localStorage.getItem(SAVED_GROUPS_EXPANDED_KEY);
    if (!raw) {
      return {};
    }
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const result: Partial<Record<GroupKey, boolean>> = {};
    for (const [key, value] of Object.entries(parsed)) {
      if (typeof value === "boolean") {
        result[key as GroupKey] = value;
      }
    }
    return result;
  } catch {
    return {};
  }
}

const expanded = ref<Partial<Record<GroupKey, boolean>>>(loadExpanded());

watch(
  expanded,
  (value) => {
    localStorage.setItem(SAVED_GROUPS_EXPANDED_KEY, JSON.stringify(value));
  },
  { deep: true },
);

const collapsingKey = ref<GroupKey | null>(null);

// Cooldown between group toggles so the expand fade-in (300ms opacity
// transition) settles before a collapse can start; otherwise a fast re-click
// collapses cards that are still mid-fade, making the overlap very visible.
const TOGGLE_SETTLE_MS = 350;
// While a collapse/expand animates, scrolling would shift the parallax x of
// the moving cards, so scrolling is locked until the rail FLIP settles.
const SCROLL_UNLOCK_DELAY_MS = 700;
let toggleUnlockAt = 0;

function toggleLocked() {
  return collapsingKey.value !== null || Date.now() < toggleUnlockAt;
}

function isCollapsed(key: GroupKey) {
  return expanded.value[key] === false;
}

function isStarred(instance: Instance) {
  return (instance.config.group ?? []).includes("starred");
}

const groups = computed<InstanceGroup[]>(() => {
  const base = filteredInstances.value;
  const favorites = base.filter(isStarred);
  const result: InstanceGroup[] = [];

  if (favorites.length > 0) {
    result.push({
      key: "starred",
      title: t("game.instances.groupTitle.favorites"),
      instances: favorites,
    });
  }

  if (groupMode.value === "loader") {
    for (const loader of LOADER_GROUPS) {
      const members = base.filter(
        (instance) =>
          (instance.config.runtime.mod_loader_type?.toLowerCase() ?? "vanilla") === loader.key,
      );
      if (members.length > 0) {
        result.push({ key: loader.key, title: loader.title, instances: members });
      }
    }
  } else if (base.length > 0) {
    result.push({ key: "all", title: t("game.instances.groupTitle.all"), instances: base });
  }

  return result;
});

async function toggleGroup(key: GroupKey) {
  if (toggleLocked()) return;
  const startedAt = Date.now();
  toggleUnlockAt = startedAt + TOGGLE_SETTLE_MS;
  scrollViewRef.value?.lockScroll();
  try {
    if (isCollapsed(key)) {
      expanded.value[key] = true;
      scrollViewRef.value?.clearGroupHeight(key);
    } else {
      collapsingKey.value = key;
      try {
        await scrollViewRef.value?.collapseGroup(key);
      } finally {
        collapsingKey.value = null;
      }
      expanded.value[key] = false;
    }
  } finally {
    window.setTimeout(() => scrollViewRef.value?.unlockScroll(), SCROLL_UNLOCK_DELAY_MS);
  }
}

const sortLabel = computed(
  () => sortOptions.value.find((x) => x.key === sortMode.value)?.label ?? "",
);
const groupLabel = computed(
  () => groupOptions.value.find((x) => x.key === groupMode.value)?.label ?? "",
);

const searchQuery = ref("");

const filteredInstances = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query) return instanceStore.instances;
  return instanceStore.instances.filter((instance) =>
    [
      instance.config.name,
      instance.config.runtime.minecraft,
      instance.config.runtime.mod_loader_type ?? "vanilla",
    ].some((field) => field.toLowerCase().includes(query)),
  );
});

function navigate(direction: -1 | 1) {
  const visibleInstances: Instance[] = [];
  for (const group of groups.value) {
    if (!isCollapsed(group.key)) {
      visibleInstances.push(...group.instances);
    }
  }
  if (visibleInstances.length === 0) return;
  const index = visibleInstances.findIndex(
    (instance) => instance.id === instanceStore.currentInstance?.id,
  );
  const nextIndex = Math.max(0, Math.min(visibleInstances.length - 1, index + direction));
  selectInstance(visibleInstances[nextIndex]);
}

function onKeyDown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null;
  if (
    target &&
    (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)
  ) {
    return;
  }
  switch (event.key) {
    case "j":
    case "ArrowDown":
      navigate(1);
      event.preventDefault();
      break;
    case "k":
    case "ArrowUp":
      navigate(-1);
      event.preventDefault();
      break;
  }
}

const overlaid = computed(
  () =>
    Object.values(useShowContent().value).find((value) => value === true) ||
    useInstanceSettings().value,
);

const backgroundImagesSrc = ref<Record<string, string>>({});

const backgroundImagesShow = ref<Record<string, boolean>>({});

async function getBackgroundSrc(id: string) {
  const backgroundPath = await getBackgroundPath(id);
  return convertFileSrc(backgroundPath) + "?t=" + Date.now();
}
</script>

<style lang="less" scoped>
.instances-list {
  height: 100%;
  width: fit-content;
  margin-left: auto;
  transform: translateX(320px);
  overflow: visible;

  .gap-top {
    height: 132px;
  }
  .gap-bottom {
    height: 100px;
  }
  .instance {
    position: relative;
    border: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);
    border-left: 16px solid rgba(var(--ctp-surface1-rgb), 0.8);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    padding: 8px 16px;
    border-radius: 8px;
    margin-top: 2px;
    width: 480px;
    height: 56px;
    opacity: 0.6;
    transition:
      border-left 200ms ease,
      margin 200ms ease,
      transform 100ms linear,
      opacity 300ms ease;

    &:not(.card-container.current .instance):active {
      transform: scale(0.99);
    }

    &:not(.card-container.current .instance):hover {
      background: rgba(var(--ctp-surface0-rgb), 0.8);
    }
    img.instance-background {
      mask-image: linear-gradient(to left, black 0%, transparent 100%);
      width: calc(100% - 200px);
      height: 100%;
      object-fit: cover;
      position: absolute;
      top: 0;
      left: 100px;
      border-radius: 0 8px 8px 0;
    }
    p {
      font-size: 15.2px;
    }
    .details {
      margin-top: 4px;
      .tag {
        font-size: 10px;
        border-radius: 100px;
        padding: 1px 6px;
        font-weight: 700;
        display: inline-flex;
        align-items: center;
        width: fit-content;
      }
      .tag.quilt {
        background: var(--ctp-mauve);
        color: var(--ctp-text-inverse);
      }
      .tag.fabric {
        background: var(--ctp-yellow);
        color: var(--ctp-text-inverse);
      }
      .tag.forge {
        background: var(--ctp-blue);
        color: var(--ctp-text-inverse);
      }
      .tag.neoforge {
        background: var(--ctp-peach);
        color: var(--ctp-text-inverse);
      }
      .tag.vanilla {
        background: var(--ctp-green);
        color: var(--ctp-text-inverse);
      }
      .last-play,
      .minecraft-version {
        font-size: 10px;
        margin-left: 8px;
        font-weight: 500;
        .label {
          opacity: 0.8;
          font-weight: 300;
        }
      }
    }
  }
  .instance.visible {
    opacity: 1;
  }

  .card-container {
    transition: margin 100ms linear;
    margin-top: 0px;
    margin-bottom: 0px;
    will-change: transform;
  }
  .card-container.current {
    .instance {
      border-left: 16px solid rgba(var(--ctp-lavender-rgb), 0.8);
      margin-left: -20px;
      transform: scale(1.03);
    }
  }
  .group-card {
    will-change: transform;
  }
  .group {
    --group-accent-rgb: var(--ctp-lavender-rgb);
    border: 1px solid rgba(var(--group-accent-rgb), 0.8);
    border-left: 20px solid rgba(var(--group-accent-rgb), 0.8);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    border-radius: 8px;
    padding: 8px 16px;
    margin-top: 2px;
    transform: translateX(-40px);
    width: 480px;
    height: 40px;
    display: flex;
    align-items: center;
    font-size: 14px;
    position: relative;
    transition: background 200ms ease;

    &:hover {
      background: rgba(var(--ctp-surface0-rgb), 0.8);
    }

    svg {
      position: absolute;
      left: -16px;
      transition: transform 200ms ease;
    }

    :deep(svg path) {
      stroke: var(--ctp-text-inverse);
    }

    &:not(.collapsed) svg {
      transform: rotate(90deg);
    }

    .group-count {
      margin-left: 8px;
      font-size: 10px;
      font-weight: 500;
      background: rgb(var(--group-accent-rgb));
      color: var(--ctp-text-inverse);
      border-radius: 100px;
      padding: 1px 6px;
    }
  }
  .group-card[data-group="starred"] .group {
    --group-accent-rgb: var(--ctp-yellow-rgb);
  }
  .group-card[data-group="all"] .group {
    --group-accent-rgb: var(--ctp-lavender-rgb);
  }
  .group-card[data-group="quilt"] .group {
    --group-accent-rgb: var(--ctp-mauve-rgb);
  }
  .group-card[data-group="fabric"] .group {
    --group-accent-rgb: var(--ctp-yellow-rgb);
  }
  .group-card[data-group="neoforge"] .group {
    --group-accent-rgb: var(--ctp-peach-rgb);
  }
  .group-card[data-group="forge"] .group {
    --group-accent-rgb: var(--ctp-blue-rgb);
  }
  .group-card[data-group="vanilla"] .group {
    --group-accent-rgb: var(--ctp-green-rgb);
  }
  .card-container.collapsing .instance {
    opacity: 0;
  }
  .instance-list-placeholder {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    right: 320px;
    width: 380px;
    height: fit-content;
    display: flex;
    align-items: center;
    flex-direction: column;
    .title {
      font-size: 16px;
      margin-top: 12px;
    }
    .desc {
      font-size: 14px;
      margin-top: 8px;
    }
  }
  .instance-list-placeholder-transition-enter-active {
    transition:
      transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1),
      opacity 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .instance-list-placeholder-transition-leave-active {
    transition:
      transform 0.25s cubic-bezier(0.215, 0.61, 0.355, 1),
      opacity 0.25s cubic-bezier(0.215, 0.61, 0.355, 1);
  }
  .instance-list-placeholder-transition-enter-from,
  .instance-list-placeholder-transition-leave-to {
    opacity: 0;
    transform: translateY(-50%) scale(0.9);
  }
}
</style>
