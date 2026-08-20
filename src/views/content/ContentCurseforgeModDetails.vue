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
      <div class="details" v-if="modInfo">
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
    return;
  }
  const response = await getMod(modId.value);
  modInfo.value = response.data;
  console.log(modInfo.value);
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
.content-curseforge-mod-details {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;
  .title {
    width: 100%;
    background: var(--ctp-mantle);
    height: 52px;
    padding: 0 32px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 8px;
  }
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

    .markdown-body {
      margin-top: 16px;
      padding: 16px;
      background: var(--ctp-surface0);
      border-radius: 8px;
      font-size: 14px;
      line-height: 1.6;
      color: var(--ctp-text);
      word-wrap: break-word;

      :deep(h1),
      :deep(h2),
      :deep(h3),
      :deep(h4),
      :deep(h5),
      :deep(h6) {
        margin-top: 24px;
        margin-bottom: 16px;
        font-weight: 600;
        line-height: 1.25;
        color: var(--ctp-text);
      }

      :deep(h1) {
        font-size: 2em;
        padding-bottom: 0.3em;
        border-bottom: 1px solid var(--ctp-surface1);
      }

      :deep(h2) {
        font-size: 1.5em;
        padding-bottom: 0.3em;
        border-bottom: 1px solid var(--ctp-surface1);
      }

      :deep(h3) {
        font-size: 1.25em;
      }

      :deep(h4) {
        font-size: 1em;
      }

      :deep(p) {
        margin-top: 0;
        margin-bottom: 16px;
      }

      :deep(a) {
        color: var(--ctp-blue);
        text-decoration: none;

        &:hover {
          text-decoration: underline;
        }
      }

      :deep(code) {
        padding: 0.2em 0.4em;
        margin: 0;
        font-size: 85%;
        background: var(--ctp-surface1);
        border-radius: 6px;
        font-family: monospace;
      }

      :deep(pre) {
        margin-top: 0;
        margin-bottom: 16px;
        padding: 16px;
        overflow: auto;
        font-size: 85%;
        line-height: 1.45;
        background: var(--ctp-mantle);
        border-radius: 8px;

        code {
          padding: 0;
          margin: 0;
          background: transparent;
          border-radius: 0;
        }
      }

      :deep(blockquote) {
        margin: 0 0 16px 0;
        padding: 0 1em;
        color: var(--ctp-overlay2);
        border-left: 0.25em solid var(--ctp-surface1);
      }

      :deep(ul),
      :deep(ol) {
        margin-top: 0;
        margin-bottom: 16px;
        padding-left: 2em;
      }

      :deep(li) {
        margin-top: 0.25em;
      }

      :deep(li + li) {
        margin-top: 0.25em;
      }

      :deep(table) {
        display: block;
        width: max-content;
        max-width: 100%;
        overflow: auto;
        margin-top: 0;
        margin-bottom: 16px;
        border-spacing: 0;
        border-collapse: collapse;
      }

      :deep(tr) {
        background: var(--ctp-surface0);
        border-top: 1px solid var(--ctp-surface1);
      }

      :deep(th),
      :deep(td) {
        padding: 6px 13px;
        border: 1px solid var(--ctp-surface1);
      }

      :deep(th) {
        font-weight: 600;
        background: var(--ctp-mantle);
      }

      :deep(hr) {
        height: 0.25em;
        padding: 0;
        margin: 24px 0;
        background-color: var(--ctp-surface1);
        border: 0;
      }

      :deep(img) {
        max-width: 100%;
        border-radius: 8px;
      }

      :deep(input[type="checkbox"]) {
        margin-right: 0.5em;
      }
    }
  }
}
</style>
