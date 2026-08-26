<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-packs">
    <ScrollView>
      <div class="title">
        <AppIcon name="package" fill="none" :size="22"></AppIcon>
        <p>{{ t("overlays.content.packs.title") }}</p>
        <div class="select-source">
          <button
            class="modrinth"
            :class="{ active: currentView === 'modrinth' }"
            @click="currentView = 'modrinth'">
            <Modrinth style="width: 24px; padding: 3px"></Modrinth>
          </button>
          <button
            class="curseforge"
            :class="{ active: currentView === 'curseforge' }"
            @click="currentView = 'curseforge'">
            <CurseForge style="width: 24px"></CurseForge>
          </button>
        </div>
      </div>
      <ContentPacksModrinth v-if="currentView === 'modrinth'" />
      <ContentPacksCurseforge v-if="currentView === 'curseforge'" />
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import Modrinth from "@/assets/images/modrinth.svg";
import CurseForge from "@/assets/images/curseforge.svg";
import AppIcon from "@/components/AppIcon.vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import ScrollView from "@/components/ScrollView.vue";
import ContentPacksModrinth from "./ContentPacksModrinth.vue";
import ContentPacksCurseforge from "./ContentPacksCurseforge.vue";

const { t } = useI18n();

const currentView = ref("modrinth" as "modrinth" | "curseforge");
</script>

<style lang="less" scoped>
@import "./styles/title-bar.less";

.content-packs {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;

  .title {
    margin-bottom: 0;

    .select-source {
      display: flex;
      align-items: center;
      border-radius: 4px;
      overflow: hidden;
      gap: 4px;
      margin-left: auto;

      button {
        appearance: none;
        border: none;
        background: none;
        width: 40px;
        height: 32px;
        display: flex;
        align-items: center;
        border-radius: 4px;
        justify-content: center;
      }

      button:hover {
        background: var(--ctp-surface0);
      }

      button:active {
        background: var(--ctp-surface1);
      }

      button.modrinth {
        fill: rgba(var(--ctp-text-rgb), 0.7);
      }

      button.active.modrinth {
        fill: var(--ctp-green);
      }

      button.curseforge {
        fill: rgba(var(--ctp-text-rgb), 0.7);
      }

      button.active.curseforge {
        fill: var(--ctp-peach);
      }

      button.active {
        background: var(--ctp-surface2);
      }
    }
  }
}
</style>
