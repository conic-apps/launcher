<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view">
    <InstanceSummary ref="instance-summary"></InstanceSummary>
    <InstancesList ref="instance-list"></InstancesList>
    <GameFooterBar ref="game-footer-bar"></GameFooterBar>
    <Transition name="game-content">
      <div
        class="game-content-wrapper instance-settings-wrapper"
        v-if="showInstanceSettings"
        @click.self="showInstanceSettings = false"
        :class="{ show: showInstanceSettings }">
        <div class="game-content-container">
          <InstanceSetting></InstanceSetting>
        </div>
      </div>
    </Transition>
    <Transition name="game-content">
      <div
        class="game-content-wrapper"
        v-if="showGameContent.saves"
        @click.self="showGameContent.saves = false"
        :class="{ show: showGameContent.saves }">
        <div class="game-content-container">
          <ContentSaves></ContentSaves>
        </div>
      </div>
    </Transition>
    <Transition name="game-content">
      <div
        class="game-content-wrapper"
        v-if="showGameContent.mods"
        @click.self="showGameContent.mods = false"
        :class="{ show: showGameContent.mods }">
        <div class="game-content-container">
          <ContentMods></ContentMods>
        </div>
      </div>
    </Transition>
    <Transition name="game-content">
      <div
        class="game-content-wrapper"
        v-if="showGameContent.resourcepacks"
        @click.self="showGameContent.resourcepacks = false"
        :class="{ show: showGameContent.resourcepacks }">
        <div class="game-content-container">
          <ContentResourcepacks></ContentResourcepacks>
        </div>
      </div>
    </Transition>
    <Transition name="game-content">
      <div
        class="game-content-wrapper"
        v-if="showGameContent.screenshots"
        @click.self="showGameContent.screenshots = false"
        :class="{ show: showGameContent.screenshots }">
        <ContentScreenshots></ContentScreenshots>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import InstanceSummary from "./game/InstanceSummary.vue";
import InstancesList from "./game/InstancesList.vue";
import GameFooterBar from "./game/Footer.vue";
import { onMounted, useTemplateRef } from "vue";
import gsap from "gsap";
import InstanceSetting from "./game/InstanceSetting.vue";
import { useInstanceSettings } from "./game/useGameView";
import { useShowContent } from "./content/useContent";
import ContentSaves from "./content/ContentSaves.vue";
import ContentMods from "./content/ContentMods.vue";
import ContentResourcepacks from "./content/ContentResourcepacks.vue";
import ContentScreenshots from "./content/ContentScreenshots.vue";

const instanceSummary = useTemplateRef("instance-summary");
const instanceList = useTemplateRef("instance-list");
const gameFooterBar = useTemplateRef("game-footer-bar");

const showInstanceSettings = useInstanceSettings();
const showGameContent = useShowContent();

onMounted(async () => {
  const intro = gsap.timeline({ paused: true });
  await Promise.all([gameFooterBar.value!.ready, instanceList.value!.ready]);
  intro.add(gameFooterBar.value!.playIntro());
  intro.add(instanceSummary.value!.playIntro(), "<");
  intro.add(instanceList.value!.playIntro(), "<0.05");
  intro.play();
});
</script>

<style lang="less" scoped>
.game-view {
  width: 100%;
  height: 100%;
  position: relative;
}
.game-content-wrapper {
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #00000080;
  z-index: 114514;

  .game-content-container {
    background: var(--ctp-base);
    width: calc(100% - 150px);
    height: 100%;
    display: flex;
    flex-direction: column;
  }
}

.game-content-wrapper.instance-settings-wrapper .game-content-container {
  width: calc(100% - 200px);
}

.game-content-enter-from,
.game-content-leave-to {
  background: #00000000;
}

.game-content-enter-to,
.game-content-leave-from {
  background: #00000000;
}

.game-content-enter-active,
.game-content-leave-active {
  transition: all 200ms ease;
}

.game-content-leave-active {
  transition-delay: 100ms;
}

.game-content-enter-from .game-content-container,
.game-content-leave-to .game-content-container {
  transform: translateY(100%);
}

.game-content-enter-active .game-content-container {
  transition: all 400ms cubic-bezier(0, 0.47, 0.25, 1);
}

.game-content-leave-active .game-content-container {
  transition: all 280ms cubic-bezier(0.47, 0, 1, 0.75);
}

.game-content-enter-to,
.game-content-leave-from {
  background: #00000080;
}
</style>
