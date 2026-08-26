<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mod-loader-chooser">
    <SettingGroup style="padding: 0"> </SettingGroup>
    <div class="fabric-version version-list" v-if="modLoaderType === 'Fabric'">
      <BaseListItem
        v-for="(fabricVersion, index) in fabricVersions"
        :key="index"
        :title="fabricVersion.loader.version"
        logo="1"
        :clickable="true"
        @click="setModloaderVersion(fabricVersion.loader.version)">
        <template #icon>
          <img style="width: 100%; height: 100%" src="@/assets/images/fabric.webp" alt="" />
        </template>
        <template #subtitle> </template>
      </BaseListItem>
    </div>
    <div class="quilt-version version-list" v-else-if="modLoaderType === 'Quilt'">
      <BaseListItem
        v-for="(quiltVersion, index) in quiltVersions"
        :key="index"
        :title="quiltVersion.loader.version"
        logo="1"
        :clickable="true"
        @click="setModloaderVersion(quiltVersion.loader.version)">
        <template #icon>
          <img style="width: 100%; height: 100%" src="@/assets/images/quilt.svg?url" alt="" />
        </template>
        <template #subtitle> </template>
      </BaseListItem>
    </div>
    <div class="forge-version version-list" v-else-if="modLoaderType === 'Forge'">
      <BaseListItem
        v-for="(forgeVersion, index) in forgeVersions"
        :key="index"
        :title="forgeVersion"
        logo="1"
        :clickable="true"
        @click="setModloaderVersion(forgeVersion.split('-').slice(1).join('-'))">
        <template #icon>
          <img style="width: 100%; height: 100%" src="@/assets/images/forge.svg?url" alt="" />
        </template>
        <template #subtitle> </template>
      </BaseListItem>
    </div>
    <div class="neoforge-version" v-else-if="modLoaderType === 'Neoforge'">
      <BaseListItem
        v-for="(neoforgeVersion, index) in neoforgeVersions"
        :key="index"
        :title="neoforgeVersion"
        logo="1"
        :clickable="true"
        @click="setModloaderVersion(neoforgeVersion)">
        <template #icon>
          <img style="width: 100%; height: 100%" src="@/assets/images/neoforge.png" alt="" />
        </template>
        <template #subtitle> </template>
      </BaseListItem>
    </div>
  </div>
</template>

<script setup lang="ts">
import BaseListItem from "@/components/BaseListItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { FabricLoaderArtifact, QuiltVersion } from "@conic/install";

const modLoaderVersion = defineModel<string>("version");
defineProps<{
  modLoaderType: "None" | "Quilt" | "Fabric" | "Neoforge" | "Forge";
  fabricVersions: FabricLoaderArtifact[] | null;
  quiltVersions: QuiltVersion[] | null;
  neoforgeVersions: string[] | null;
  forgeVersions: string[] | null;
}>();
const emits = defineEmits(["back"]);

function setModloaderVersion(version: string) {
  modLoaderVersion.value = version;
  emits("back");
}
</script>

<style lang="less" scoped>
.mod-loader-chooser {
  height: 100%;
  width: 100%;
  .mod-loader-type {
    background: var(--ctp-surface0);
  }

  .version-list {
    margin: 0 8px;
    width: calc(100% - 16px);
    border-radius: 8px;
    overflow: hidden;
  }
}
</style>
