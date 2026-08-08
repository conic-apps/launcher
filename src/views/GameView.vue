<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view" data-tauri-drag-region>
    <InstanceSummary ref="instance-summary"></InstanceSummary>
    <InstancesList ref="instance-list"></InstancesList>
    <GameFooterBar ref="game-footer-bar"></GameFooterBar>
    <Transition name="instance-settings">
      <div
        class="instance-settings-wrapper"
        v-if="showInstanceSettings"
        @click="showInstanceSettings = false"
        :class="{ show: showInstanceSettings }">
        <div class="instance-settings-container" @click.stop>
          <InstanceSetting></InstanceSetting>
        </div>
      </div>
    </Transition>
    <Transition name="game-content">
      <div
        class="game-content-wrapper"
        v-if="showGameContent"
        @click="showGameContent = false"
        :class="{ show: showGameContent }">
        <Content @click.stop class="game-content-container"></Content>
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
import Content from "./game/Content.vue";
import { useShowContent } from "./game/useContent";

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
.instance-settings-wrapper {
  overflow: hidden;
  width: 100vw;
  height: 100vh;
  background: #00000080;
  position: absolute;
  top: 0;
  left: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 16px;

  .instance-settings-container {
    width: 640px;
    height: 100vh;
    overflow-y: auto;
    position: absolute;
    left: 0;
    transform: translateX(0);
    background: rgba(var(--ctp-base-rgb), 1);
    border-radius: 16px 0 0 16px;
    padding: 24px 16px;
  }
}

.instance-settings-enter-from,
.instance-settings-leave-to {
  background: #00000000;
}

.instance-settings-enter-to,
.instance-settings-leave-from {
  background: #00000000;
}

.instance-settings-enter-active,
.instance-settings-leave-active {
  transition: all 200ms ease;
}

.instance-settings-leave-active {
  transition-delay: 100ms;
}

.instance-settings-enter-from .instance-settings-container,
.instance-settings-leave-to .instance-settings-container {
  transform: translateX(-640px);
}

.instance-settings-enter-active .instance-settings-container {
  transition: all 400ms cubic-bezier(0, 0.47, 0.25, 1);
}

.instance-settings-leave-active .instance-settings-container {
  transition: all 280ms cubic-bezier(0.47, 0, 1, 0.75);
}

.instance-settings-enter-to,
.instance-settings-leave-from {
  background: #00000080;
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
