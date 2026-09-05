<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="current-instance" :class="{ introPlayed }">
    <div class="row-1" ref="row1" style="opacity: 0">
      <p class="title">{{ currentInstance?.config.name ?? "no instance selected!" }}</p>
    </div>
    <div class="row-2" ref="row2" style="opacity: 0">
      <p>
        <span> {{ t("game.summary.minecraftVersion") }} </span>
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
          {{ currentInstance.config.runtime.mod_loader_type }} {{ t("game.summary.version") }}
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
        <span>{{ formatPlayTime(playtimeCache[currentInstance.id] ?? 0, playTimeFormatter) }}</span>
      </p>
    </div>
    <div class="row-3" ref="row3" style="opacity: 0">
      <div class="launch-buttons" ref="launchButtons">
        <button
          class="launch-button"
          @click="navigationStore.navigate('launch')"
          :class="{ disabled: !currentInstance }">
          <AppIcon name="play" fill="#fff" style="margin-right: 4px"></AppIcon>
          {{ t("game.summary.startGame") }}
        </button>
        <button class="launch-sub-button" @click.stop="toggleLaunchMenu">
          <span class="chevron" ref="launchMenuChevron">
            <AppIcon
              name="chevron-down"
              stroke="#ffffff"
              fill="#ffffff"
              style="color: #fff"
              :size="16"></AppIcon>
          </span>
        </button>
        <Transition
          :css="false"
          @before-enter="onBeforeEnter"
          @enter="onEnter"
          @after-enter="onAfterEnter"
          @enter-cancelled="onEnterCancelled"
          @before-leave="onBeforeLeave"
          @leave="onLeave"
          @after-leave="onAfterLeave"
          @leave-cancelled="onLeaveCancelled">
          <ul
            class="launch-menu-dropdown"
            v-if="launchMenuOpened"
            @click="launchMenuOpened = false">
            <li class="dropdown-option" @click="repairAndLaunch">
              <AppIcon name="build" :size="14"></AppIcon>
              <span>{{ t("game.summary.repairAndLaunch") }}</span>
            </li>
          </ul>
        </Transition>
      </div>
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
    <div class="current-instance-contents" ref="contents" :class="{ disabled: !currentInstance }">
      <div @click="useShowContent().value.saves = true" ref="saves" style="opacity: 0">
        <div>
          <AppIcon name="save"></AppIcon>
          <span class="type">{{ t("game.summary.saves") }}</span>
        </div>
        <div>
          <div
            class="content-img"
            v-for="(folderName, index) in Object.keys(contentStore.gameContent.saves ?? {}).slice(
              0,
              5,
            )"
            :key="index">
            <img
              v-if="iconCache[folderName]"
              :src="iconCache[folderName]"
              alt="world icon"
              width="64px"
              height="64px" />
            <img
              v-else
              src="@/assets/images/Unknown_server.webp"
              alt="world icon"
              width="64px"
              height="64px" />
          </div>
          <span class="count" v-if="contentStore.loading.saves">
            <BaseLoading :size="16" :strokeWidth="6" :gap="6"></BaseLoading>
          </span>
          <span class="count" v-else
            >{{ Object.keys(contentStore.gameContent.saves ?? {}).length }}
            {{ t("game.summary.countUnit") }}</span
          >
        </div>
      </div>
      <div
        @click="useShowContent().value.mods = true"
        ref="mods"
        style="opacity: 0"
        :class="{
          disabled:
            !currentInstance?.config.runtime.mod_loader_type ||
            !currentInstance.config.runtime.mod_loader_version,
        }">
        <div>
          <AppIcon name="extension-puzzle" />
          <span class="type">{{ t("game.summary.mods") }}</span>
        </div>
        <div>
          <div
            class="content-img"
            v-for="(mod, index) in (contentStore.gameContent.mods ?? []).slice(0, 5)"
            :key="index">
            <img v-if="mod.icon" :src="mod.icon" alt="mod icon" width="64px" height="64px" />
            <img
              v-else
              src="@/assets/images/Unknown_server.webp"
              alt="world icon"
              width="64px"
              height="64px" />
          </div>
          <span class="count" v-if="contentStore.loading.mods">
            <BaseLoading :size="16" :strokeWidth="6" :gap="6"></BaseLoading>
          </span>
          <span class="count" v-else
            >{{ (contentStore.gameContent.mods ?? []).length }}
            {{ t("game.summary.countUnit") }}</span
          >
        </div>
      </div>
      <div
        @click="useShowContent().value.resourcepacks = true"
        ref="resourcepacks"
        style="opacity: 0">
        <div>
          <AppIcon name="folder" />
          <span class="type">{{ t("game.summary.resourcePacks") }}</span>
        </div>
        <div>
          <div
            class="content-img"
            v-for="(pack, index) in (contentStore.gameContent.resourcepacks ?? []).slice(0, 5)"
            :key="index">
            <img
              v-if="pack.icon"
              :src="pack.icon"
              alt="resourcepack icon"
              width="64px"
              height="64px" />
            <img
              v-else
              src="@/assets/images/Unknown_server.webp"
              alt="world icon"
              width="64px"
              height="64px" />
          </div>
          <span class="count" v-if="contentStore.loading.resourcepacks">
            <BaseLoading :size="16" :strokeWidth="6" :gap="6"></BaseLoading>
          </span>
          <span class="count" v-else
            >{{ (contentStore.gameContent.resourcepacks ?? []).length }}
            {{ t("game.summary.countUnit") }}</span
          >
        </div>
      </div>
      <div
        @click="useShowContent().value.screenshots = true"
        ref="screenshots"
        style="opacity: 0"
        :class="{ disabled: !(contentStore.gameContent.screenshots ?? []).length }">
        <div>
          <AppIcon name="images-outline" />
          <span class="type">{{ t("game.summary.screenshots") }}</span>
        </div>
        <div>
          <div
            class="content-img"
            v-for="(src, index) in (contentStore.gameContent.screenshots ?? [])
              .slice(0, 5)
              .map((path) => convertFileSrc(path))"
            :key="index">
            <img :src="src" alt="screenshot icon" width="64px" height="64px" />
          </div>
          <span class="count" v-if="contentStore.loading.screenshots">
            <BaseLoading :size="16" :strokeWidth="6" :gap="6"></BaseLoading>
          </span>
          <span class="count" v-else
            >{{ (contentStore.gameContent.screenshots ?? []).length }}
            {{ t("game.summary.countUnit") }}</span
          >
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseLoading from "@/components/BaseLoading.vue";
import { useInstanceStore } from "@/store/instance";
import { computed, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import {
  calculatePlaytime,
  formatLastPlayed,
  formatPlayTime,
  removeInstallLock,
  updateInstance,
} from "@conic/instance";
import { useNavigationStore } from "@/store/navigation";
import { getInstanceRoot } from "@conic/folder";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
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

// Removes the install lock of the current instance and launches the game, so
// the launch pipeline re-runs the installation flow first.
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

const rowElements = {
  row1: useTemplateRef("row1"),
  row2: useTemplateRef("row2"),
  row3: useTemplateRef("row3"),
};

const gameContentElements = {
  saves: useTemplateRef("saves"),
  mods: useTemplateRef("mods"),
  resourcepacks: useTemplateRef("resourcepacks"),
  screenshots: useTemplateRef("screenshots"),
};

const introPlayed = ref(false);

const playIntro = () => {
  return gsap
    .timeline({
      onComplete: () => {
        introPlayed.value = true;
      },
    })
    .fromTo(
      Object.values(rowElements).map((elementRef) => elementRef.value),
      { opacity: 0, x: -50 },
      {
        opacity: 1,
        x: 0,
        duration: 0.33,
        stagger: 0.03,
        ease: "power3.out",
      },
    )
    .fromTo(
      Object.values(gameContentElements).map((elementRef) => elementRef.value),
      {
        opacity: 0,
        x: -50,
      },
      {
        opacity: 1,
        x: 0,
        duration: 0.33,
        stagger: 0.03,
        ease: "power3.out",
      },
      "<+0.03",
    );
};

defineExpose({ playIntro });
</script>

<style lang="less" scoped>
.current-instance {
  position: absolute;
  top: 45%;
  transform: translateY(-50%);
  margin-left: 48px;

  &:not(.introPlayed) {
    pointer-events: none;
  }

  .row-1 {
    display: flex;
    align-items: center;

    .current-instance-icon {
      width: 40px;
      height: 40px;
      border-radius: calc(var(--card-icon-border-radius) + 4px);
      background: var(--card-icon-background);
    }

    .title {
      font-size: 38px;
    }
  }

  .row-2 {
    display: flex;
    align-items: center;
    margin-top: 16px;

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

    .open-instance-setting-button {
      appearance: none;
      background: none;
      border: none;
      width: 32px;
      height: 32px;
      margin-right: 16px;
    }

    .open-instance-setting-button:active {
      opacity: 0.9;
      transform: scale(0.95);
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

    .launch-buttons {
      position: relative;
      display: flex;
      align-items: center;

      .chevron {
        display: inline-flex;
        align-items: center;
      }
    }

    .launch-button {
      appearance: none;
      border: none;
      color: #fff;
      width: 128px;
      height: 42px;
      font-size: 15px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 8px 0 0 8px;
      background: rgb(114, 135, 253);
    }

    .launch-sub-button {
      appearance: none;
      border: none;
      background: #ffffff4f;
      height: 42px;
      width: 24px;
      border-radius: 0 8px 8px 0;
      background: rgb(114, 135, 253);
      margin-left: 2px;
    }

    .launch-menu-dropdown {
      position: absolute;
      top: calc(100% + 4px);
      left: 0;
      min-width: 100%;
      padding: 8px 10px;
      border-radius: var(--dialog-border-radius);
      border: var(--controllers-border);
      background: var(--ctp-base);
      box-shadow: 0px 0px 10px #4500611d;
      z-index: 100000;
      list-style: none;

      .dropdown-option {
        height: 26px;
        padding: 0 8px;
        display: flex;
        align-items: center;
        gap: 8px;
        margin: 4px 0;
        border-radius: var(--controllers-border-radius);
        font-size: 12px;
        list-style: none;
        white-space: nowrap;
        transition: all 30ms ease;

        &:hover {
          background: #ffffff1f;
        }

        &:active {
          background: #ffffff15;
        }
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
  }

  .current-instance-contents {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 16px;
    width: fit-content;
    padding: 4px 4px;
    border-radius: 12px;
    gap: 4px;

    > div {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 8px 16px;
      border-radius: 8px;
      background: rgba(var(--ctp-surface0-rgb), 0.7);
      transition: all 100ms ease;
      font-size: 14px;
      width: 400px;

      &:hover {
        background: var(--ctp-surface0);
        transition: none;
      }

      &:active {
        background: var(--ctp-surface1);
        transform: scale(0.95);
      }
    }
    > div > div {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    > div > div > div.content-img {
      display: flex;
      gap: 1px;
      img {
        width: 20px;
        height: 20px;
        border-radius: 10000px;
        border: 1px solid rgba(var(--ctp-lavender-rgb), 0.8);
      }
    }
    > div.disabled,
    > div.disabled * {
      pointer-events: none;
      opacity: 0.6;
    }
  }
  .current-instance-contents.disabled * {
    opacity: 0.6;
    pointer-events: none;
  }
}
</style>
