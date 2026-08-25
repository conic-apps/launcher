<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-curseforge-pack-details">
    <ScrollView>
      <div class="title">
        <AppIcon name="package"></AppIcon>
        <p>整合包信息</p>
      </div>
      <div class="details" v-if="modInfo">
        <div class="header">
          <div class="icon">
            <img :src="modInfo.logo?.url" alt="pack icon" />
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
import { computed, onMounted, ref, watch } from "vue";
import ScrollView from "@/components/ScrollView.vue";
import { getMod, getModDescription, Mod as CurseforgeMod } from "@conic/curseforge";
import { useShowContentDetails } from "./useContent";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { openUrl } from "@tauri-apps/plugin-opener";

const { curseforgeCache: curseforgeTranslations, translateCurseforgeSummaries } =
  useDescriptionTranslation();

const modInfo = ref(null as null | CurseforgeMod);
const modDescription = ref("");
const unsafeHtmlRe = /<\s*(script|style)\b/i;
const safeDescription = computed(() =>
  unsafeHtmlRe.test(modDescription.value)
    ? "Unable to display this unsafe content"
    : modDescription.value,
);

const modId = computed(() => useShowContentDetails().value.curseforge.pack);

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
    return;
  }
  const response = await getMod(modId.value);
  modInfo.value = response.data;
  if (modInfo.value) {
    void translateCurseforgeSummaries([modInfo.value.id]);
    const descResponse = await getModDescription(modInfo.value.id, { markup: true });
    modDescription.value = descResponse.data ?? "";
  }
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

.content-curseforge-pack-details {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;

  .details {
    padding: 8px 24px;

    .header {
      display: flex;

      .icon {
        margin-right: 16px;

        img {
          width: 88px;
          height: 88px;
          overflow: hidden;
          border-radius: 12px;
        }
      }

      .info {
        display: flex;
        flex-direction: column;
        justify-content: center;
      }

      .project-name {
        font-size: 24px;
        font-weight: 600;
        margin-bottom: 8px;
      }

      .metadata {
        display: flex;
        align-items: center;
        font-size: 14px;
        margin-bottom: 8px;
        gap: 12px;

        span {
          display: flex;
          align-items: center;

          svg {
            margin-right: 4px;
          }
        }

        :deep(svg) {
          stroke: var(--ctp-mauve);
          fill: var(--ctp-mauve);
        }

        :deep(path) {
          stroke: var(--ctp-mauve);
          fill: var(--ctp-mauve);
        }

        > * + *::before {
          content: "";
          display: inline-block;
          width: 1px;
          height: 16px;
          background-color: var(--ctp-overlay2);
          margin-right: 12px;
        }
      }

      .description {
        font-size: 14px;
      }
    }
  }
}
</style>
