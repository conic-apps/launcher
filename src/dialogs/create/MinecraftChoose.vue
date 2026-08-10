<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="minecraft-choose">
    <div class="filter">
      <p>版本分类</p>
      <base-select
        :options="['releases', 'snapshot', 'old', 'special']"
        :display-name="['正式版', '快照版', '远古版', '愚人节版']"
        v-model="showVersionType"></base-select>
    </div>
    <div class="list">
      <BaseListItem
        v-for="(version, index) in filteredVersions"
        :key="index"
        :title="`Minecraft ${version.id}`"
        logo="1"
        :clickable="true"
        @click="selectVersion(version.id)"
        :buttons="['about']"
        @click-about="clickAbout(version.id)"
        :description="parseTime(version.releaseTime)">
        <template #icon>
          <img
            v-if="version.type == `release`"
            style="width: 100%; height: 100%; margin-right: 8px; opacity: 0.8"
            src="@/assets/images/minecraft.webp"
            alt="" />
          <img
            v-else-if="version.type == `snapshot`"
            style="width: 100%; height: 100%; margin-right: 8px"
            src="@/assets/images/Command_Block.webp"
            alt="" />
          <img
            v-else
            style="width: 100%; height: 100%; margin-right: 8px"
            src="@/assets/images/Ancient_Debris.webp"
            alt="" />
        </template>
      </BaseListItem>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import BaseListItem from "@/components/BaseListItem.vue";
import { getMinecrafVersionManifest, VersionManifest } from "@conic/install";
import { openUrl } from "@tauri-apps/plugin-opener";
import BaseSelect from "@/components/BaseSelect.vue";

const versions = ref<VersionManifest>();
getMinecrafVersionManifest()
  .then((res) => {
    if (res) {
      versions.value = res;
    } else {
      throw "get_version_list failed!";
    }
  })
  .catch((err) => {
    console.error(err);
  });

const showVersionType = ref<"releases" | "snapshot" | "old" | "special">("releases");

const filteredVersions = computed(() => {
  if (showVersionType.value === "releases") {
    return versions.value?.versions.filter((version) => version.type === "release");
  } else if (showVersionType.value === "snapshot") {
    return versions.value?.versions.filter((version) => version.type === "snapshot");
  } else if (showVersionType.value === "old") {
    return versions.value?.versions.filter((version) => version.type.includes("old"));
  } else if (showVersionType.value === "special") {
    // TODO: Add spacial version to VersionManifest
    return versions.value?.versions.filter((version) => version.type.includes("special"));
  } else {
    return versions.value?.versions.filter((version) => version.type === "release");
  }
});
function parseTime(time: string) {
  const date = new Date(time);
  return `发布于 ${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`;
}

const model = defineModel();
const emits = defineEmits(["back"]);

function selectVersion(mcversion: string) {
  model.value = mcversion;
  emits("back");
}

function clickAbout(version: string) {
  openUrl(`https://zh.minecraft.wiki/w/Java%E7%89%88${version}`);
}
</script>

<style lang="less" scoped>
.minecraft-choose {
  height: calc(100% - 64px);
  width: 100%;
  margin: 12px 14px;
  width: calc(100% - 28px);
  display: flex;
  flex-direction: column;
  padding-right: 8px;
  .filter {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--ctp-surface0);
    padding: 8px 16px;
    font-size: 13px;
    border-radius: 8px;
    margin-bottom: 8px;
  }
  .list {
    overflow: auto;
    height: 100%;
    border: 1px solid rgba(0, 0, 0, 0.16);
    :first-child {
      border-radius: 8px 8px 0 0;
    }
    :last-child {
      border-radius: 0 0 8px 8px;
    }
  }
}
</style>
