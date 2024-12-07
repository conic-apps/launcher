<template>
  <div class="assets">
    <tabs
      :tabs="
        components.map((n) => {
          return i18n.t(n.name);
        })
      "
      :active="activeTab"
      :icons="
        components.map((n) => {
          return n.icon;
        })
      "
      @choose-tab="chooseTab">
    </tabs>
    <Transition :name="transitionName" mode="out-in">
      <component
        :is="currentComponent"
        style="padding: 16px 8px; width: 100%; height: fit-content"
        @update-instance-list="$emit('update-instance-list')"></component>
    </Transition>
    <!-- <worlds :show="show.worlds" :datas="saves" :instance-name="instance.config.name" @close="show.worlds = false"> -->
    <!-- </worlds> -->
    <!-- <mods :show="show.mods" :datas="mods" :instance-name="instance.config.name" @close="show.mods = false"></mods> -->
    <!-- <resourcepacks :show="show.resourcepacks" :datas="resourcepacks" :instance-name="instance.config.name" -->
    <!--   @close="show.resourcepacks = false"></resourcepacks> -->
    <!-- <shaderpacks :show="show.shaderpacks" :datas="shaderpacks" :instance-name="instance.config.name" -->
    <!--   @close="show.shaderpacks = false"> -->
    <!-- </shaderpacks> -->
  </div>
</template>

<script setup lang="ts">
import { computed, markRaw, ref } from "vue";
import Tabs from "@/components/Tabs.vue";
import Info from "./Info.vue";
import Worlds from "./Worlds.vue";
import Mods from "./Mods.vue";
import Packs from "./Packs.vue";
import Settings from "./Settings.vue";
import { useI18n } from "vue-i18n";

const i18n = useI18n();

defineEmits(["update-instance-list"]);

const components = ref([
  {
    name: "game.assets.info",
    icon: "circle-info",
    component: markRaw(Info),
  },
  {
    name: "game.assets.worlds",
    icon: "map",
    component: markRaw(Worlds),
  },
  {
    name: "game.assets.mods",
    icon: "puzzle-piece",
    component: markRaw(Mods),
  },
  {
    name: "game.assets.packs",
    icon: "file-zipper",
    component: markRaw(Packs),
  },
  {
    name: "game.assets.settings",
    icon: "gear",
    component: markRaw(Settings),
  },
]);
const activeTab = ref(0);

function chooseTab(tab: number) {
  if (activeTab.value < tab) {
    transitionName.value = "slide-left";
  } else {
    transitionName.value = "slide-right";
  }
  activeTab.value = tab;
}
const currentComponent = computed(() => {
  return components.value[activeTab.value].component;
});

const transitionName = ref("slide-left");
</script>

<style lang="less" scoped>
.assets {
  margin-top: 14px;
  width: 100%;
  overflow-x: hidden;
}

.assets > div {
  display: flex;
  width: 100%;
}

.assets > div.first-row {
  margin-right: 5px;
}

.assets > div.second-row {
  margin-left: 5px;
}
</style>
