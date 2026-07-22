<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="home-view">
    <InstanceSummary></InstanceSummary>
    <InstanceDetails style="margin-top: 16px"></InstanceDetails>
    <InstanceDrawer></InstanceDrawer>
  </div>
</template>

<script setup lang="ts">
import InstanceSummary from "./home/InstanceSummary.vue";
import InstanceDetails from "./home/InstanceDetails.vue";
import InstanceDrawer from "./home/InstanceDrawer.vue";
</script>

<style lang="less" scoped>
.home-view {
  width: 100%;
  height: 100%;
  padding: 32px;
  display: flex;
  flex-direction: column;
  align-items: center;
  overflow-y: auto;
  padding-bottom: 96px;
}
</style>
