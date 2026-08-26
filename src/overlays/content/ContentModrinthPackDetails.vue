<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-modrinth-pack-details">
    <ScrollView>
      <div class="title">
        <AppIcon name="package"></AppIcon>
        <p>{{ t("overlays.content.common.packInfo") }}</p>
      </div>
      <div class="search-status" v-if="loading && !projectInfo">
        <BaseLoading :size="32" :gap="8" :strokeWidth="4"></BaseLoading>
      </div>
      <div class="details" v-else-if="projectInfo">
        <div class="header">
          <div class="icon">
            <img :src="projectInfo.icon_url" alt="pack icon" />
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
import BaseLoading from "@/components/BaseLoading.vue";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ScrollView from "@/components/ScrollView.vue";
import ScrollViewHorizontal from "@/components/ScrollViewHorizontal.vue";
import { getProject, getTeamMembers, Project, TeamMembers } from "@conic/modrinth";
import { useShowContentDetails } from "./useContent";
import { useDescriptionTranslation } from "./useDescriptionTranslation";
import { openUrl } from "@tauri-apps/plugin-opener";
import { marked } from "marked";

const { t } = useI18n();

const { modrinthCache: modrinthTranslations, translateModrinthDescriptions } =
  useDescriptionTranslation();

const projectInfo = ref(null as null | Project);
const loading = ref(true);
const readmeHtml = computed(() => {
  if (!projectInfo.value?.body) return "";
  return marked.parse(projectInfo.value.body) as string;
});

const translatedDescription = computed(() => {
  if (!projectInfo.value) return "";
  return modrinthTranslations.get(projectInfo.value.id) ?? projectInfo.value.description ?? "";
});

const projectId = computed(() => useShowContentDetails().value.modrinth.pack);
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
    loading.value = false;
    return;
  }
  loading.value = true;
  projectInfo.value = await getProject(projectId.value);
  if (projectInfo.value) {
    void translateModrinthDescriptions([projectInfo.value.id]);
  }
  loading.value = false;
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
@import "./styles/title-bar.less";
@import "./styles/markdown-body.less";
@import "./styles/details.less";

.content-modrinth-pack-details {
  &:extend(.content-details all);

  .search-status {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px 0;
    font-size: 13px;
    color: var(--ctp-subtext0);
  }

  .details .gallery {
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
</style>
