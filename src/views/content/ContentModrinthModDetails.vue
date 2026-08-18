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
            <p class="description" v-if="projectInfo.description">{{ projectInfo.description }}</p>
          </div>
        </div>
        <div class="section readme">
          <!-- NOTE: 在此处渲染markdown -->
        </div>
      </div>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import { computed, onMounted, ref, watch } from "vue";
import ScrollView from "@/components/ScrollView.vue";
import { getProject, getTeamMembers, Project, TeamMembers } from "@conic/modrinth";
import { useShowContentDetails } from "./useContent";
import { openUrl } from "@tauri-apps/plugin-opener";

const projectInfo = ref(null as null | Project);

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
  } catch (error) {
    return null;
  }
}

const formattedGithubRepo = computed(() => formatGithubRepo(projectInfo.value?.source_url ?? ""));
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
  }
}
</style>
