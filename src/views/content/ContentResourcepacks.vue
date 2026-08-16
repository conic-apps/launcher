<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-resourcepacks">
    <ScrollView>
      <div class="title">
        <AppIcon name="palette"></AppIcon>
        <p>资源包列表</p>
        <!-- FIXME: reverse lavender color, button hover -->
        <div class="select-source">
          <button :class="{ active: currentView === 'local' }" @click="currentView = 'local'">
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
      <ContentResourcepacksLocal v-if="currentView === 'local'" />
      <ContentResourcepacksModrinth v-if="currentView === 'modrinth'" />
      <ContentResourcepacksCurseforge v-if="currentView === 'curseforge'" />
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import Modrinth from "@/assets/images/modrinth.svg";
import CurseForge from "@/assets/images/curseforge.svg";
import AppIcon from "@/components/AppIcon.vue";
import { ref } from "vue";
import ScrollView from "@/components/ScrollView.vue";
import ContentResourcepacksLocal from "./ContentResourcepacksLocal.vue";
import ContentResourcepacksModrinth from "./ContentResourcepacksModrinth.vue";
import ContentResourcepacksCurseforge from "./ContentResourcepacksCurseforge.vue";

const currentView = ref("local" as "local" | "modrinth" | "curseforge");
</script>

<style lang="less" scoped>
.content-resourcepacks {
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
