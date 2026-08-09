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
      <div
        class="card-container"
        :class="{ current: instance.id === instanceStore.currentInstance.id }"
        v-for="instance in filteredInstances"
        style="opacity: 0"
        :key="instance.id">
        <div class="instance" @click="selectInstance(instance)" :data-id="instance.id">
          <p v-if="instance.id === LATEST_RELEASE_INSTANCE_ID">
            {{ $t("game.latestRelease") }}
          </p>
          <p v-else-if="instance.id === LATEST_SNAPSHOT_INSTANCE_ID">
            {{ $t("game.latestSnapshot") }}
          </p>
          <p v-else>{{ instance.config.name }}</p>
          <div class="details">
            <span
              :class="`tag ${instance.config.runtime.mod_loader_type.toLowerCase()}`"
              v-if="instance.config.runtime.mod_loader_type"
              >{{ instance.config.runtime.mod_loader_type }}</span
            >
            <span class="tag vanilla" v-else>Vanilla</span>
            <span class="last-play"
              ><span class="label">上次运行：</span>
              <span v-if="instance.last_played">{{
                formatLastPlayed(instance.last_played, zhCN)
              }}</span>
              <span v-else>从未运行</span>
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
      <div class="gap-bottom"></div>
    </InstancesListScrollView>
  </div>
</template>

<script setup lang="ts">
import { useInstanceStore } from "@/store/instance";
import {
  formatLastPlayed,
  getBackgroundPath,
  Instance,
  InstanceSort,
  LATEST_RELEASE_INSTANCE_ID,
  LATEST_SNAPSHOT_INSTANCE_ID,
  zhCN,
} from "@conic/instance";
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import InstancesListToolBar from "./InstancesListToolBar.vue";
import InstancesListScrollView from "./InstancesListScrollView.vue";
import { useShowContent } from "./useContent";
import { useInstanceSettings } from "./useGameView";
import gsap from "gsap";

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
  await nextTick();
  scrollViewRef.value?.scrollTo(instanceStore.currentInstance.id, false);
  requestAnimationFrame(() => {
    scrollViewRef.value?.reflow().then(() => resolveReady());
  });
  Object.values(instanceStore.instances).forEach(async (instance) => {
    backgroundImagesSrc.value[instance.id] = await getBackgroundSrc(instance.id);
  });
});

const playIntro = () => {
  const tl = gsap.timeline();
  const listIntro = scrollViewRef.value?.playIntro();
  tl.add(toolbarRef.value?.playIntro() ?? gsap.timeline());
  if (listIntro) tl.add(listIntro, "<+0.1145141919810");
  return tl;
};

defineExpose({ playIntro, ready });

export type SortMode = "name" | "version" | "playtime" | "lastplay";
const sortMode = ref<SortMode>("playtime");
const sortOptions: { key: SortMode; label: string }[] = [
  { key: "name", label: "名称" },
  { key: "version", label: "版本" },
  { key: "playtime", label: "游玩时间" },
  { key: "lastplay", label: "最后运行" },
];

const SORT_MODE_TO_SORT: Record<SortMode, InstanceSort> = {
  name: "Name",
  version: "Version",
  playtime: "Playtime",
  lastplay: "LastPlayed",
};

watch(sortMode, (mode) => {
  instanceStore.setSort(SORT_MODE_TO_SORT[mode]);
});

function selectSort(mode: SortMode) {
  sortMode.value = mode;
}

export type GroupMode = "all" | "none" | "loader";
const groupMode = ref<GroupMode>("all");
const groupOptions: { key: GroupMode; label: string }[] = [
  { key: "all", label: "全部实例" },
  { key: "none", label: "未分组" },
  { key: "loader", label: "模组加载器" },
];

function selectGroup(mode: GroupMode) {
  groupMode.value = mode;
}

const sortLabel = computed(() => sortOptions.find((x) => x.key === sortMode.value)?.label ?? "");
const groupLabel = computed(() => groupOptions.find((x) => x.key === groupMode.value)?.label ?? "");

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

const overlaid = computed(() => useShowContent().value || useInstanceSettings().value);

const backgroundImagesSrc = ref<Record<string, string>>({});

const backgroundImagesShow = ref<Record<string, boolean>>({});

async function getBackgroundSrc(id: string) {
  const backgroundPath = await getBackgroundPath(id);
  return convertFileSrc(backgroundPath);
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
        font-weight: 500;
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
}
</style>
