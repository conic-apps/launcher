<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-view" data-tauri-drag-region>
    <InstanceSummary ref="instance-summary"></InstanceSummary>
    <InstancesList ref="instance-list"></InstancesList>
    <GameFooterBar ref="game-footer-bar"></GameFooterBar>
  </div>
</template>

<script setup lang="ts">
import InstanceSummary from "./game/InstanceSummary.vue";
import InstancesList from "./game/InstancesList.vue";
import GameFooterBar from "./game/GameFooterBar.vue";
import { onMounted, useTemplateRef } from "vue";
import gsap from "gsap";

const instanceSummary = useTemplateRef("instance-summary");
const instanceList = useTemplateRef("instance-list");
const gameFooterBar = useTemplateRef("game-footer-bar");

onMounted(async () => {
  const intro = gsap.timeline({ paused: true });
  await Promise.all([gameFooterBar.value!.ready]);
  intro.add(gameFooterBar.value!.playIntro());
  intro.play();
});
</script>

<style lang="less" scoped>
.game-view {
  width: 100%;
  height: 100%;
  position: relative;
}
</style>
