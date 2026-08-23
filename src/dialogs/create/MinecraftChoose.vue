<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="minecraft-choose">
    <div class="filter">
      <p>版本分类</p>
      <!-- TODO: Special version support -->
      <base-select
        :options="['releases', 'snapshot', 'old']"
        :display-name="['正式版', '快照版', '远古版']"
        v-model="showVersionType"></base-select>
    </div>
    <div class="list">
      <Transition>
        <div class="loading" v-if="loading">
          <BaseLoading></BaseLoading>
        </div>
      </Transition>
      <ScrollView>
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
              style="width: 100%; height: 100%; opacity: 0.8"
              src="@/assets/images/minecraft.webp"
              alt="" />
            <img
              v-else-if="version.type == `snapshot`"
              style="width: 100%; height: 100%"
              src="@/assets/images/Command_Block.webp"
              alt="" />
            <img
              v-else
              style="width: 100%; height: 100%"
              src="@/assets/images/Ancient_Debris.webp"
              alt="" />
          </template>
        </BaseListItem>
      </ScrollView>
    </div>
    <BaseButton @click="dialogStore.createInstance.visible = false" style="margin-top: 8px"
      >取消</BaseButton
    >
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import BaseListItem from "@/components/BaseListItem.vue";
import { getMinecrafVersionManifest, VersionManifest } from "@conic/install";
import { openUrl } from "@tauri-apps/plugin-opener";
import BaseSelect from "@/components/BaseSelect.vue";
import { useDialogStore } from "@/store/dialog";
import BaseButton from "@/components/BaseButton.vue";

import ScrollView from "@/components/ScrollView.vue";
import BaseLoading from "@/components/BaseLoading.vue";
const dialogStore = useDialogStore();

const versions = ref<VersionManifest>();
const loading = ref(false);
onMounted(() => {
  loading.value = true;
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
    })
    .finally(() => {
      loading.value = false;
    });
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
  return `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`;
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
  height: calc(100% - 42px);
  width: 100%;
  display: flex;
  flex-direction: column;
  padding: 8px;
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
    overflow: hidden;
    height: 100%;
    border: 1px solid rgba(0, 0, 0, 0.16);
    border-radius: 8px;
    position: relative;
    .loading {
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #0000004b;
      z-index: 1;
    }
  }
}
</style>
