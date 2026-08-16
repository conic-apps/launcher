<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-mods">
    <ScrollView>
      <div class="title">
        <AppIcon name="extension-puzzle"></AppIcon>
        <p>模组列表</p>
        <div class="select-source">
          <button
            class="local"
            :class="{ active: currentView === 'local' }"
            @click="currentView = 'local'">
            <AppIcon name="file-tray-full-outline"></AppIcon>
          </button>
          <button :class="{ active: currentView === 'modrinth' }" @click="currentView = 'modrinth'">
            <Modrinth fill="var(--ctp-green)" style="width: 24px; padding: 3px"></Modrinth>
          </button>
          <button
            :class="{ active: currentView === 'curseforge' }"
            @click="currentView = 'curseforge'">
            <CurseForge fill="var(--ctp-peach)" style="width: 24px"></CurseForge>
          </button>
        </div>
      </div>
      <ContentModsLocal v-if="currentView === 'local'" />
      <ContentModsModrinth v-if="currentView === 'modrinth'" />
      <ContentModsCurseforge v-if="currentView === 'curseforge'" />
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import Modrinth from "@/assets/images/modrinth.svg";
import CurseForge from "@/assets/images/curseforge.svg";
import AppIcon from "@/components/AppIcon.vue";
import { ref } from "vue";
import ScrollView from "@/components/ScrollView.vue";
import ContentModsLocal from "./ContentModsLocal.vue";
import ContentModsModrinth from "./ContentModsModrinth.vue";
import ContentModsCurseforge from "./ContentModsCurseforge.vue";

const currentView = ref("local" as "local" | "modrinth" | "curseforge");
</script>

<style lang="less" scoped>
.content-mods {
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
}
</style>
