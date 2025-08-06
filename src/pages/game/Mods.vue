<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="mods">
    <div class="mod-list">
      <list-item
        v-for="(mod, index) in mods"
        :key="index"
        :title="mod.name ? mod.name! : mod.version!"
        :logo="mod.icon ? mod.icon : '1'"
        :click-able="false"
        :buttons="['circle-info', 'folders', 'trash-can']">
        <template #icon v-if="!mod.icon">
          <img src="@/assets/images/Unknown_server.webp" alt="" style="width: 100%; height: 100%" />
        </template>
        <template #subtitle v-if="!mod.name">
          <tag
            text="无法识别"
            :color="['255', '129', '120']"
            :background="true"
            :border="true"></tag>
        </template>
        <!-- <template #subtitle v-else-if="mod.type == 'lib'"> -->
        <!--   <tag text="支持库" :color="['200', '200', '0']" :border="true"></tag> -->
        <!-- </template> -->
        {{ mod.description }}
      </list-item>
    </div>
  </div>
</template>

<script setup lang="ts">
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";
import { useInstanceStore } from "@/store/instance";
import { ref, watch } from "vue";

type Mod = {
  name: string;
  description: string | null;
  version: string | null;
  depends: any; // TODO: show depends
  authors: {
    name: string;
    contact: Map<string, string>;
  };
  license: string[] | null;
  icon: string | null;
};

const instanceStore = useInstanceStore();

const mods = ref<Mod[]>([]);

watch(
  instanceStore.currentInstance,
  () => {
    // todo: scan mod folder
  },
  {
    immediate: true,
  },
);
</script>

<style lang="less" scoped></style>
