<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-curseforge-mod-details">
    <ScrollView>
      <div class="title">
        <AppIcon name="extension-puzzle"></AppIcon>
        <p>模组信息</p>
      </div>
      <div class="search-status" v-if="loading && !modInfo">
        <BaseLoading :size="32" :gap="8" :strokeWidth="4"></BaseLoading>
      </div>
      <div class="details" v-else-if="modInfo">
        <div class="header">
          <div class="icon">
            <img :src="modInfo.logo?.url" alt="mod icon" />
          </div>
          <div class="info">
            <p class="project-name" v-if="modInfo.name">{{ modInfo.name }}</p>
            <p class="metadata">
              <span v-if="modInfo.links?.sourceUrl">
                <AppIcon name="code-slash-outline"></AppIcon>
                <a
                  :href="modInfo.links.sourceUrl"
                  @click.prevent="openUrl(modInfo.links.sourceUrl)"
                  >{{ modInfo.links.sourceUrl }}</a
                >
              </span>
              <span>
                <AppIcon name="download"></AppIcon>
                {{ modInfo.downloadCount }}
              </span>
              <span v-if="modInfo.thumbsUpCount">
                <AppIcon name="heart-outline"></AppIcon>
                {{ modInfo.thumbsUpCount }}
              </span>
            </p>
            <p class="description" v-if="translatedDescription">{{ translatedDescription }}</p>
          </div>
        </div>
        <div
          class="section readme markdown-body"
          v-if="modDescription"
          v-html="safeDescription"
          @click="onReadmeClick"></div>
      </div>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseLoading from "@/components/BaseLoading.vue";
import { computed, onMounted, ref, watch } from "vue";
import ScrollView from "@/components/ScrollView.vue";
import { getMod, getModDescription, Mod as CurseforgeMod } from "@conic/curseforge";
import { useShowContentDetails } from "./useContent";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { openUrl } from "@tauri-apps/plugin-opener";

const { curseforgeCache: curseforgeTranslations, translateCurseforgeSummaries } =
  useDescriptionTranslation();

const modInfo = ref(null as null | CurseforgeMod);
const loading = ref(true);
const modDescription = ref("");
const unsafeHtmlRe = /<\s*(script|style)\b/i;
const safeDescription = computed(() =>
  unsafeHtmlRe.test(modDescription.value)
    ? "Unable to display this unsafe content"
    : modDescription.value,
);

const modId = computed(() => useShowContentDetails().value.curseforge.mod);

const translatedDescription = computed(() => {
  if (!modInfo.value) return "";
  return curseforgeTranslations.get(modInfo.value.id) ?? modInfo.value.summary ?? "";
});

onMounted(async () => {
  await refreshModInfo();
});

watch(modId, async () => {
  await refreshModInfo();
});

async function refreshModInfo() {
  if (!modId.value) {
    modInfo.value = null;
    modDescription.value = "";
    loading.value = false;
    return;
  }
  loading.value = true;
  const response = await getMod(modId.value);
  modInfo.value = response.data;
  if (modInfo.value) {
    void translateCurseforgeSummaries([modInfo.value.id]);
    const descResponse = await getModDescription(modInfo.value.id, { markup: true });
    modDescription.value = descResponse.data ?? "";
  }
  loading.value = false;
}

function onReadmeClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  if (target.tagName === "A") {
    event.preventDefault();
    const href = (target as HTMLAnchorElement).href;
    if (href) openUrl(href);
  }
}
</script>

<style lang="less" scoped>
@import "./styles/title-bar.less";
@import "./styles/markdown-body.less";
@import "./styles/details.less";

.content-curseforge-mod-details {
  &:extend(.content-details all);

  .search-status {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px 0;
    font-size: 13px;
    color: var(--ctp-subtext0);
  }
}
</style>
