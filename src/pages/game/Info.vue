<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-info">
    <Playtime></Playtime>
    <div class="title">Recent Played Worlds</div>
    <div style="overflow-x: auto; white-space: nowrap; padding-bottom: 8px">
      <div class="recent-world" v-for="(world, index) in recentWorlds" :key="index">
        <img :src="world.image" />
        <!-- TODO: Style -->
        <p class="name">{{ world.name }}</p>
        <p class="desc">{{ world.latestPlay }}</p>
      </div>
    </div>
    <div class="title">Recent Played Servers</div>
    <div style="overflow-x: auto; white-space: nowrap; padding-bottom: 8px">
      <div class="recent-server" v-for="(server, index) in recentServers" :key="index">
        <img :src="server.image" />
        <!-- TODO: Style -->
        <p class="name">{{ server.name }}</p>
        <p class="desc">{{ server.latestPlay }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import Playtime from "./Playtime.vue";

const recentWorlds = ref([
  {
    image: "/Unknown_server.webp",
    name: "New World",
    latestPlay: "Nov 13, 2014",
  },
]);
const recentServers = ref([
  {
    image: "/Unknown_server.webp",
    name: "A Minecraft Server",
    latestPlay: "Nov 13, 2014",
  },
]);

for (let index = 0; index < 10; index++) {
  recentWorlds.value.push({
    image: "/Unknown_server.webp",
    name: "New World",
    latestPlay: "Nov 13, 2014",
  });
  recentServers.value.push({
    image: "/Unknown_server.webp",
    name: "A Minecraft Server",
    latestPlay: "Nov 13, 2014",
  });
}
</script>

<style lang="less" scoped>
.instance-info {
  display: flex;
  flex-direction: column;

  div.title {
    margin: 16px 0;
    font-size: 18px;
  }

  div.recent-world,
  div.recent-server {
    width: 120px;
    height: 100px;
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border-radius: var(--card-border-radius);
    border: var(--card-border);
    background: var(--card-background);
    overflow: hidden;

    margin-right: 8px;

    img {
      width: 60px;
      height: 60px;
      border-radius: var(--card-border-radius);
    }

    p.name {
      font-size: 14px;
    }

    p.desc {
      font-size: 12px;
    }
  }
}
</style>
