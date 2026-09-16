<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="current-instance" :class="{ introPlayed }">
    <div class="summary-card" ref="summaryCard" style="opacity: 0">
      <div class="row-1">
        <p class="title">{{ currentInstance?.config.name ?? "no instance selected!" }}</p>
      </div>
      <div class="row-2">
        <p>
          <span>Minecraft</span>
          <span>{{ currentInstance?.config.runtime.minecraft ?? "--" }}</span>
        </p>
        <div
          class="line"
          v-if="
            currentInstance?.config.runtime.mod_loader_type &&
            currentInstance?.config.runtime.mod_loader_version
          "></div>
        <p
          v-if="
            currentInstance?.config.runtime.mod_loader_type &&
            currentInstance?.config.runtime.mod_loader_version
          ">
          <span>
            {{ currentInstance.config.runtime.mod_loader_type }}
          </span>
          <span>{{ currentInstance.config.runtime.mod_loader_version }}</span>
        </p>
        <div class="line"></div>
        <p>
          <span>{{ t("game.summary.lastPlayedDate") }}</span>
          <span v-if="!currentInstance">--</span>
          <span v-else-if="currentInstance.last_played">{{
            formatLastPlayed(currentInstance.last_played, timeFormatter)
          }}</span>
          <span v-else>{{ t("game.summary.neverPlayed") }}</span>
        </p>
        <div
          class="line"
          v-if="
            currentInstance &&
            playtimeCache[currentInstance.id] &&
            playtimeCache[currentInstance.id] > 0
          "></div>
        <AppIcon
          name="time"
          :size="22"
          style="margin-right: 2px"
          v-if="
            currentInstance &&
            playtimeCache[currentInstance.id] &&
            playtimeCache[currentInstance.id] > 0
          "></AppIcon>
        <p
          v-if="
            currentInstance &&
            playtimeCache[currentInstance.id] &&
            playtimeCache[currentInstance.id] > 0
          ">
          <span>{{ t("game.summary.playTime") }}</span>
          <span>{{
            formatPlayTime(playtimeCache[currentInstance.id] ?? 0, playTimeFormatter)
          }}</span>
        </p>
      </div>
      <div class="row-3">
        <button
          class="launch-button"
          @click="navigationStore.navigate('launch')"
          :class="{ disabled: !currentInstance }">
          <AppIcon name="rocket" fill="var(--ctp-text-inverse)" :size="18"></AppIcon>
          <span style="color: var(--ctp-text-inverse)">启动</span>
        </button>
        <div class="actions" :class="{ disabled: !currentInstance }">
          <button class="action-button" @click="openInstanceFolder">
            <AppIcon name="folder"></AppIcon>
          </button>
          <button class="action-button" @click="toggleStarred">
            <AppIcon :name="isStarred ? 'star' : 'star-outline'"></AppIcon>
          </button>
          <button class="action-button" @click="useInstanceSettings().value = true">
            <AppIcon name="settings"></AppIcon>
          </button>
        </div>
      </div>
    </div>

    <div class="tabs" ref="tabsEl" style="opacity: 0">
      <button
        class="tab"
        v-for="tab in tabs"
        :key="tab.key"
        :ref="(el: unknown) => setTabRef(tab.key, el)"
        :class="{ active: activeTab === tab.key }"
        @click="onTabClick(tab.key)">
        {{ tab.label }}
      </button>
      <div class="tab-indicator" ref="tabIndicator"></div>
    </div>

    <div class="content-area" ref="contentAreaEl" style="opacity: 0">
      <template v-if="activeContentLoading">
        <div class="state-wrapper">
          <BaseLoading :size="32" :gap="8" :strokeWidth="4"></BaseLoading>
        </div>
      </template>
      <template v-else-if="activeContentEmpty">
        <div class="state-wrapper">
          <BaseNotFound :show="true" :description="activeEmptyDesc"></BaseNotFound>
        </div>
      </template>
      <template v-else>
        <div class="content-list">
          <template v-if="activeTab === 'saves'">
            <div class="list-item" v-for="[folder, save] in savesList" :key="folder">
              <img class="item-thumb" :src="iconCache[folder] ?? unknownImg" alt="" />
              <div class="item-main">
                <div class="item-title-line">
                  <span class="item-title">{{ save.Data.LevelName }}</span>
                  <span class="cheats" v-if="save.Data.allowCommands">{{
                    t("overlays.content.saves.cheats")
                  }}</span>
                </div>
                <span class="item-sub">{{ folder }}</span>
              </div>
              <div class="item-meta">
                <div class="meta-item">
                  <span class="meta-label">{{ t("game.summary.lastPlayedTime") }}</span>
                  <span class="meta-value">{{
                    save.Data.LastPlayed
                      ? formatLastPlayed(save.Data.LastPlayed, timeFormatter)
                      : "--"
                  }}</span>
                </div>
                <div class="meta-item">
                  <span class="meta-label">{{ t("game.summary.gameMode") }}</span>
                  <span class="meta-value">{{ formatGameType(save.Data.GameType) }}</span>
                </div>
              </div>
            </div>
          </template>
          <template v-else-if="activeTab === 'mods'">
            <div class="list-item" v-for="(mod, idx) in modsList" :key="idx">
              <img class="item-thumb" :src="mod.icon ?? unknownImg" alt="" />
              <div class="item-main">
                <span class="item-title">{{ mod.name }}</span>
                <span class="item-sub" v-if="mod.authors.length"
                  >by {{ mod.authors.map((a) => a.name).join(", ") }}</span
                >
              </div>
              <div class="item-meta" v-if="mod.version">
                <span class="meta-value">{{ mod.version }}</span>
              </div>
            </div>
          </template>
          <template v-else-if="activeTab === 'resourcepacks'">
            <div class="list-item" v-for="(pack, idx) in rpList" :key="idx">
              <img class="item-thumb" :src="pack.icon ?? unknownImg" alt="" />
              <div class="item-main">
                <span class="item-title">{{ pack.name }}</span>
              </div>
            </div>
          </template>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseLoading from "@/components/BaseLoading.vue";
import BaseNotFound from "@/components/BaseNotFound.vue";
import { useInstanceStore } from "@/store/instance";
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import {
  calculatePlaytime,
  formatLastPlayed,
  formatPlayTime,
  removeInstallLock,
  updateInstance,
} from "@conic/instance";
import { useNavigationStore } from "@/store/navigation";
import { getInstanceRoot } from "@conic/folder";
import { invoke } from "@tauri-apps/api/core";
import { useInstanceSettings } from "@/overlays/useInstanceSettings";
import {
  flipDropdownChevron,
  useDismissOnOutsidePointerDown,
  useDropdownTransition,
} from "./useDropdownTransition";
import { useGameContentStore } from "@/store/content";
import { useShowContent, useShowContentDetails } from "@/overlays/content/useContent";
import gsap from "gsap";
import { useI18n } from "vue-i18n";
import { getSaveIcon } from "@conic/content";
import unknownImg from "@/assets/images/Unknown_server.webp";

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

const playTimeFormatter = {
  seconds: (count: number) => t("game.time.seconds", { count }),
  minutes: (count: number) => t("game.time.minutes", { count }),
  hours: (count: number) => t("game.time.hours", { count }),
};

const instanceStore = useInstanceStore();
const navigationStore = useNavigationStore();
const contentStore = useGameContentStore();

const showInstanceSettings = useInstanceSettings();
const showContent = useShowContent();
const showContentDetails = useShowContentDetails();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});

async function openInstanceFolder() {
  if (!currentInstance.value) {
    return;
  }
  invoke("open_path", { path: await getInstanceRoot(currentInstance.value.id) });
}

const isStarred = computed(() => (currentInstance.value?.config.group ?? []).includes("starred"));

async function toggleStarred() {
  if (!currentInstance.value) return;
  const config = {
    ...currentInstance.value.config,
    group: isStarred.value
      ? (currentInstance.value.config.group ?? []).filter((group) => group !== "starred")
      : [...(currentInstance.value.config.group ?? []), "starred"],
  };
  await updateInstance(config, currentInstance.value.id);
  await instanceStore.loadInstances();
}

const launchMenuOpened = ref(false);
const launchButtons = ref<HTMLElement | null>(null);
const launchMenuChevron = ref<HTMLElement | null>(null);

useDismissOnOutsidePointerDown(launchButtons, launchMenuOpened);

function toggleLaunchMenu() {
  launchMenuOpened.value = !launchMenuOpened.value;
}

const {
  onBeforeEnter,
  onEnter,
  onAfterEnter,
  onEnterCancelled,
  onBeforeLeave,
  onLeave,
  onAfterLeave,
  onLeaveCancelled,
} = useDropdownTransition(launchMenuOpened, {
  onChange: (value) => {
    flipDropdownChevron(launchMenuChevron.value, value ? 180 : 0);
  },
});

async function repairAndLaunch() {
  if (!currentInstance.value) return;
  try {
    await removeInstallLock(currentInstance.value.id);
  } catch (error) {
    console.error(error);
    return;
  }
  currentInstance.value.installed = false;
  navigationStore.navigate("launch");
}

function onKeyDown(event: KeyboardEvent) {
  const target = event.target as HTMLElement | null;
  if (
    target &&
    (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable)
  ) {
    return;
  }
  if (showInstanceSettings.value) return;
  if (Object.values(showContent.value).some((v) => v)) return;
  const details = showContentDetails.value;
  if (
    details.modrinth.mod ||
    details.modrinth.resourcepack ||
    details.curseforge.mod ||
    details.curseforge.resourcepack
  ) {
    return;
  }
  if (event.key === "Enter") {
    navigationStore.navigate("launch");
    event.preventDefault();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeyDown);
  nextTick(updateIndicator);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeyDown);
});

const playtimeCache = ref<Record<string, number>>({});
watch(
  currentInstance,
  async (newValue) => {
    if (!newValue) return;
    const instanceId = newValue.id;
    if (!!playtimeCache.value[instanceId]) {
      return;
    }
    try {
      const playtime = await calculatePlaytime(instanceId);
      playtimeCache.value[instanceId] = playtime;
    } catch {}
  },
  { immediate: true },
);

const iconCache = ref({} as Record<string, string>);

watch(
  () => contentStore.gameContent.saves,
  async (saves) => {
    if (!saves) {
      return;
    }
    const promises = Object.keys(saves).map(async (key) => {
      try {
        if (!instanceStore.currentInstance) {
          throw "currentInstance is null";
        }
        iconCache.value[key] = await getSaveIcon(instanceStore.currentInstance.id, key);
      } catch (error) {
        console.error(error);
      }
    });
    await Promise.allSettled(promises);
  },
  { immediate: true },
);

function formatGameType(gameType: number | undefined) {
  if (gameType === 0) return t("overlays.content.saves.gameType.survival");
  if (gameType === 1) return t("overlays.content.saves.gameType.creative");
  if (gameType === 2) return t("overlays.content.saves.gameType.adventure");
  if (gameType === 3) return t("overlays.content.saves.gameType.spectator");
  return "--";
}

type TabKey = "saves" | "mods" | "resourcepacks";

const activeTab = ref<TabKey>("saves");

const tabs = computed(() => [
  { key: "saves" as const, label: t("game.summary.saves") },
  { key: "mods" as const, label: t("game.summary.mods") },
  { key: "resourcepacks" as const, label: t("game.summary.resourcePacks") },
  { key: "screenshots" as const, label: t("game.summary.screenshots") },
]);

const tabElMap: Record<string, HTMLElement> = {};
function setTabRef(key: string, el: unknown) {
  if (el && el instanceof HTMLElement) {
    tabElMap[key] = el;
  }
}

const tabIndicator = useTemplateRef("tabIndicator");

function updateIndicator() {
  const el = tabElMap[activeTab.value];
  const indicator = tabIndicator.value;
  if (!el || !indicator) return;
  indicator.style.left = `${el.offsetLeft}px`;
  indicator.style.width = `${el.offsetWidth}px`;
}

watch(activeTab, async () => {
  await nextTick();
  updateIndicator();
});

watch(tabs, async () => {
  await nextTick();
  updateIndicator();
});

function onTabClick(key: string) {
  if (key === "screenshots") {
    showContent.value.screenshots = true;
    return;
  }
  activeTab.value = key as TabKey;
}

const savesList = computed(() => Object.entries(contentStore.gameContent.saves ?? {}));
const modsList = computed(() => contentStore.gameContent.mods ?? []);
const rpList = computed(() => contentStore.gameContent.resourcepacks ?? []);

const activeContentLoading = computed(() => {
  switch (activeTab.value) {
    case "saves":
      return contentStore.loading.saves;
    case "mods":
      return contentStore.loading.mods;
    case "resourcepacks":
      return contentStore.loading.resourcepacks;
    default:
      return false;
  }
});

const activeContentEmpty = computed(() => {
  switch (activeTab.value) {
    case "saves":
      return Object.keys(contentStore.gameContent.saves ?? {}).length === 0;
    case "mods":
      return (contentStore.gameContent.mods ?? []).length === 0;
    case "resourcepacks":
      return (contentStore.gameContent.resourcepacks ?? []).length === 0;
    default:
      return false;
  }
});

const activeEmptyDesc = computed(() => {
  switch (activeTab.value) {
    case "saves":
      return t("overlays.content.saves.empty");
    case "mods":
      return t("overlays.content.mods.localEmpty");
    case "resourcepacks":
      return t("overlays.content.resourcepacks.localEmpty");
    default:
      return undefined;
  }
});

const summaryCard = useTemplateRef("summaryCard");
const tabsEl = useTemplateRef("tabsEl");
const contentAreaEl = useTemplateRef("contentAreaEl");
const introPlayed = ref(false);

const playIntro = () => {
  return gsap
    .timeline({
      onComplete: () => {
        introPlayed.value = true;
      },
    })
    .fromTo(
      summaryCard.value,
      { opacity: 0, x: -50 },
      { opacity: 1, x: 0, duration: 0.33, ease: "power3.out" },
    )
    .fromTo(
      tabsEl.value,
      { opacity: 0, x: -50 },
      { opacity: 1, x: 0, duration: 0.33, ease: "power3.out" },
      "<+0.03",
    )
    .fromTo(
      contentAreaEl.value,
      { opacity: 0, x: -50 },
      { opacity: 1, x: 0, duration: 0.33, ease: "power3.out" },
      "<+0.03",
    );
};

defineExpose({ playIntro });
</script>

<style lang="less" scoped>
.current-instance {
  position: absolute;
  display: flex;
  flex-direction: column;
  min-width: 420px;
  width: calc(50vw);
  max-width: 600px;

  &:not(.introPlayed) {
    pointer-events: none;
  }

  .summary-card {
    flex-shrink: 0;
    padding: 16px 20px;

    &::before {
      content: "";
      position: absolute;
      top: 0;
      right: 24px;
      bottom: 0;
      left: -100px;
      z-index: -1;
      background: rgba(var(--ctp-surface0-rgb), 0.4);
      backdrop-filter: blur(4px);
      transform: skew(-10deg);
      border-bottom-right-radius: 16px;
    }

    .row-1 {
      .title {
        font-size: 32px;
      }
    }

    .row-2 {
      display: flex;
      align-items: center;
      margin-top: 12px;

      > p {
        font-size: 12px;
        display: flex;
        flex-direction: column;
        align-items: initial;
        width: fit-content;
        padding: 2px 4px;

        :first-child {
          opacity: 0.8;
          font-size: 12px;
        }

        :last-child {
          margin-top: 2px;
          font-size: 15px;
        }
      }

      div.line {
        width: 1px;
        height: 26px;
        background: var(--ctp-surface2);
        margin: 0px 8px;
      }
    }

    .row-3 {
      display: flex;
      align-items: center;
      margin-top: 16px;
      position: relative;
      z-index: 1;

      .launch-button {
        appearance: none;
        border: none;
        color: #fff;
        width: fit-content;
        padding: 0 16px;
        height: 36px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 100px;
        background: var(--ctp-blue);
        gap: 8px;
        span {
          font-size: 13px;
        }
      }

      .launch-button.disabled,
      .launch-button.disabled ~ .launch-sub-button {
        opacity: 0.6;
        pointer-events: none;
      }

      .launch-button:active {
        opacity: 0.9;
      }

      div.actions {
        display: flex;
        margin-left: 16px;
        transition: transform 100ms ease;
      }

      div.actions.disabled {
        opacity: 0.6;
        pointer-events: none;
      }

      .action-button {
        appearance: none;
        border: none;
        color: var(--ctp-text);
        width: 32px;
        height: 32px;
        border-radius: 100px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin-right: 8px;
        background: none;
        transition:
          background 100ms ease,
          transform 100ms ease;

        &:hover {
          background: var(--ctp-surface1);
        }

        &:active {
          transform: scale(0.9);
          background: var(--ctp-surface0);
        }

        &:last-child {
          margin-right: 0;
        }
      }
    }
  }

  .tabs {
    position: relative;
    display: flex;
    gap: 8px;
    margin-top: 16px;
    width: fit-content;
    padding-bottom: 8px;
    margin-left: 16px;
    margin-bottom: 16px;

    .tab {
      appearance: none;
      background: none;
      border: none;
      padding: 4px 0;
      font-size: 14px;
      color: rgba(var(--default-text-color), 0.9);
      opacity: 0.4;
      transition: opacity 150ms ease;

      &.active {
        opacity: 1;
      }
    }

    .tab-indicator {
      position: absolute;
      bottom: 0;
      left: 0;
      width: 0;
      height: 2px;
      background: var(--ctp-blue);
      transition:
        left 300ms cubic-bezier(0.4, 0, 0.2, 1),
        width 300ms cubic-bezier(0.4, 0, 0.2, 1);
    }
  }

  .content-area {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;

    .state-wrapper {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .content-list {
      display: flex;
      flex-direction: column;
      gap: 4px;
      padding: 4px 0;
    }

    .list-item {
      display: flex;
      height: 64px;
      background: rgba(var(--ctp-surface0-rgb), 0.4);
      border-radius: 8px;
      overflow: hidden;
      flex-shrink: 0;

      .item-thumb {
        height: 100%;
        width: auto;
        flex-shrink: 0;
        background: var(--ctp-surface0);
      }

      .item-main {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        justify-content: center;
        padding: 0 12px;
        gap: 2px;

        .item-title-line {
          display: flex;
          align-items: baseline;
          gap: 6px;

          .item-title {
            font-size: 15px;
            font-weight: 700;
            overflow: hidden;
            white-space: nowrap;
            text-overflow: ellipsis;
          }

          .cheats {
            font-size: 9px;
            padding: 1px 6px;
            border-radius: 100px;
            background: var(--ctp-yellow);
            color: var(--ctp-text-inverse);
            flex-shrink: 0;
          }
        }

        .item-sub {
          font-size: 11px;
          opacity: 0.6;
          overflow: hidden;
          white-space: nowrap;
          text-overflow: ellipsis;
        }
      }

      .item-meta {
        flex-shrink: 0;
        width: 180px;
        padding: 8px 16px 8px 0;
        display: flex;
        flex-direction: column;
        justify-content: center;
        gap: 6px;
        align-items: flex-end;
        text-align: right;

        .meta-item {
          display: flex;
          flex-direction: column;
          align-items: flex-end;
        }

        .meta-label {
          font-size: 9px;
          opacity: 0.6;
        }

        .meta-value {
          font-size: 12px;
        }
      }
    }
  }
}
</style>
