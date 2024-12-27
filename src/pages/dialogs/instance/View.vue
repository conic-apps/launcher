<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-view">
    <div class="instance" v-for="instance in instanceStore.instances" :key="instance.config.name">
      <img src="@/assets/images/minecraft-icon.svg" />
      <p>{{ instance.config.name }}</p>
    </div>
    <div class="instance" @click="$emit('create')">
      <i class="plus"></i>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useInstanceStore } from "@/store/instance";

const instanceStore = useInstanceStore();
</script>

<style lang="less">
.instance-manager .content {
  .instance-view {
    // display: flex;
    padding-top: 10px;
  }

  .instance {
    display: inline-flex;
    width: 100px;
    height: 100px;
    align-items: center;
    flex-direction: column;
    justify-content: center;
    margin: 4px;
    border-radius: 10px;

    img {
      width: 40px;
      height: 40px;
    }
  }

  .instnace:hover {
    background: #ffffff0c;
    transition: transform 100ms cubic-bezier(0, 0, 0.2, 1);
  }

  .instance:active {
    opacity: 0.9;
    // transform: scale(0.96);
    transition: transform 100ms cubic-bezier(0, 0, 0.2, 1);
  }

  i.plus {
    font-family: "fa-pro";
    font-style: normal;
    font-size: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dotted #ffffffba;
    width: 40px;
    height: 40px;
    opacity: 0.6;
    border-radius: 10px;
  }

  p {
    text-align: center;
    margin-top: 8px;
    font-size: 15px;
    white-space: nowrap;
  }
}
</style>
