<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-modrinth-mod-details">
    <ScrollView>
      <div class="title">
        <AppIcon name="extension-puzzle"></AppIcon>
        <p>模组信息</p>
      </div>
      <div class="details" v-if="projectInfo">
        <div class="header">
          <div class="icon">
            <img :src="projectInfo.icon_url" alt="mod icon" />
          </div>
          <div class="info">
            <p class="project-name" v-if="projectInfo.title">{{ projectInfo.title }}</p>
            <p class="metadata">
              <span v-if="formattedGithubRepo && projectInfo.source_url">
                <AppIcon name="github"></AppIcon>
                <a
                  :href="projectInfo.source_url"
                  @click.prevent="openUrl(projectInfo.source_url)"
                  >{{ formattedGithubRepo }}</a
                >
              </span>
              <span v-else-if="projectInfo.source_url">
                <AppIcon name="code-slash-outline"></AppIcon> {{ projectInfo.source_url }}
                <a
                  :href="projectInfo.source_url"
                  @click.prevent="openUrl(projectInfo.source_url)"
                  >{{ projectInfo.source_url }}</a
                >
              </span>
              <span>
                <AppIcon name="download"></AppIcon>
                <!-- TODO: format numbers through vue-i18n -->
                {{ projectInfo.downloads }}
              </span>
              <span>
                <AppIcon name="heart-outline"></AppIcon>
                {{ projectInfo.followers }}
              </span>
            </p>
            <p class="description" v-if="translatedDescription">{{ translatedDescription }}</p>
          </div>
        </div>
        <div class="section readme markdown-body" v-html="readmeHtml" @click="onReadmeClick"></div>
        <div class="section gallery" v-if="projectInfo.gallery && projectInfo.gallery.length > 0">
          <ScrollViewHorizontal>
            <div class="gallery-list">
              <div v-for="(item, index) in projectInfo.gallery" :key="index" class="gallery-item">
                <img :src="item.url" :alt="item.title ?? `gallery ${index + 1}`" />
              </div>
            </div>
          </ScrollViewHorizontal>
        </div>
      </div>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { computed, onMounted, ref, watch } from "vue";
import ScrollView from "@/components/ScrollView.vue";
import ScrollViewHorizontal from "@/components/ScrollViewHorizontal.vue";
import { getProject, getTeamMembers, Project, TeamMembers } from "@conic/modrinth";
import { useShowContentDetails } from "./useContent";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { openUrl } from "@tauri-apps/plugin-opener";
import { marked } from "marked";

const { modrinthCache: modrinthTranslations, translateModrinthDescriptions } =
  useDescriptionTranslation();

const projectInfo = ref(null as null | Project);
const readmeHtml = computed(() => {
  if (!projectInfo.value?.body) return "";
  return marked.parse(projectInfo.value.body) as string;
});

const translatedDescription = computed(() => {
  if (!projectInfo.value) return "";
  return modrinthTranslations.get(projectInfo.value.id) ?? projectInfo.value.description ?? "";
});

const projectId = computed(() => useShowContentDetails().value.modrinth.mod);
const teamMembers = ref(null as TeamMembers | null);

onMounted(async () => {
  await refreshProjectInfo();
});

watch(projectId, async () => {
  await refreshProjectInfo();
});

watch(projectInfo, async (projectInfo) => {
  if (projectInfo) {
    teamMembers.value = await getTeamMembers(projectInfo.team);
  }
});

async function refreshProjectInfo() {
  if (!projectId.value) {
    projectInfo.value = null;
    return;
  }
  projectInfo.value = await getProject(projectId.value);
  console.log(projectInfo.value);
  if (projectInfo.value) {
    void translateModrinthDescriptions([projectInfo.value.id]);
  }
}

function formatGithubRepo(url: string): string | null {
  try {
    const cleanUrl = url.trim();
    const parsedUrl = new URL(cleanUrl);
    if (parsedUrl.hostname !== "github.com" && parsedUrl.hostname !== "://github.com") {
      return null;
    }
    const pathSegments = parsedUrl.pathname.split("/").filter((segment) => segment.length > 0);
    if (pathSegments.length < 2) {
      return null;
    }

    const owner = pathSegments[0];
    let repo = pathSegments[1];

    if (repo.endsWith(".git")) {
      repo = repo.slice(0, -4);
    }

    return `${owner}/${repo}`;
  } catch {
    return null;
  }
}

const formattedGithubRepo = computed(() => formatGithubRepo(projectInfo.value?.source_url ?? ""));

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
.content-modrinth-mod-details {
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

    .select-source {
      display: flex;
      align-items: center;
      border-radius: 4px;
      overflow: hidden;
      margin-left: auto;
      border: 1px solid var(--ctp-surface1);

      button {
        appearance: none;
        border: none;
        background: none;
        background: var(--ctp-surface0);
        width: 40px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-right: 1px solid var(--ctp-surface1);
      }
      button.local.active :deep(path) {
        stroke: var(--ctp-text-inverse);
      }
      button:last-child {
        border-right: none;
      }
      button.active {
        background: var(--ctp-lavender);
      }
    }
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
      :deep(*) {
        -webkit-user-select: unset;
      }

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

    .gallery {
      margin-top: 16px;
      padding: 16px;
      background: var(--ctp-surface0);
      border-radius: 8px;
      height: 252px;
      position: relative;

      .gallery-list {
        display: flex;
        align-items: center;
        gap: 12px;
        height: 100%;
        padding: 0 8px;

        .gallery-item {
          flex-shrink: 0;
          height: calc(100% - 16px);
          border-radius: 8px;
          overflow: hidden;

          img {
            height: 100%;
            width: auto;
            display: block;
            border-radius: 8px;
            user-select: none;
            -webkit-user-drag: none;
          }
        }
      }
    }
  }
}
</style>
