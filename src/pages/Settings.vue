<template>
  <div class="settings">
    <div class="rol-1">
      <ul class="settings-menu">
        <li
          @click="switchComponent(item, index)"
          :class="[activeComponentIndex == index ? 'active' : '']"
          v-for="(item, index) in components"
          :key="index">
          <i :class="`${item.icon} fa-pro`"></i>{{ $t(item.name) }}
        </li>
      </ul>
    </div>
    <div class="rol-2">
      <Transition :name="transitionName" mode="out-in">
        <KeepAlive>
          <component :is="currentComponent"></component>
        </KeepAlive>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { markRaw, ref, shallowRef, watch } from "vue";
import General from "./settings/General.vue";
import Game from "./settings/Game.vue";
import Advance from "./settings/Advance.vue";
import Appearance from "./settings/Appearance.vue";
import Download from "./settings/Download.vue";
import Accessibility from "./settings/Accessibility.vue";
import Extend from "./settings/Extend.vue";
import About from "./settings/About.vue";
import { useConfigStore } from "@/config";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
const i18n = useI18n();
const config = useConfigStore();

const components = ref([
  {
    name: "settings.general.sidebar",
    icon: "house",
    component: markRaw(General),
  },
  {
    name: "settings.game.sidebar",
    icon: "gamepad",
    component: markRaw(Game),
  },
  {
    name: "settings.advance.sidebar",
    icon: "pro-settings",
    component: markRaw(Advance),
  },
  {
    name: "settings.appearance.sidebar",
    icon: "palette",
    component: markRaw(Appearance),
  },
  {
    name: "settings.download.sidebar",
    icon: "download",
    component: markRaw(Download),
  },
  {
    name: "settings.accessibility.sidebar",
    icon: "arrows-spin",
    component: markRaw(Accessibility),
  },
  {
    name: "settings.extend.sidebar",
    icon: "cubes",
    component: markRaw(Extend),
  },
  {
    name: "settings.about.sidebar",
    icon: "circle-exclamation",
    component: markRaw(About),
  },
]);
const currentComponent = shallowRef(components.value[0].component);
let activeComponentIndex = ref(0);
let transitionName = ref("slide-up");
function switchComponent(item: any, index: number) {
  if (activeComponentIndex.value < index) {
    transitionName.value = "slide-up";
  } else {
    transitionName.value = "slide-down";
  }
  currentComponent.value = item.component;
  activeComponentIndex.value = index;
}
</script>

<style lang="less" scoped>
.settings {
  width: 100%;
  height: 100%;
  display: flex;
}

.rol-1,
.rol-2 {
  height: 100%;
}

.rol-1 {
  width: 260px;
  flex-shrink: 0;
  padding: 30px 0px 16px 24px;
}

.rol-2 {
  width: 100%;
  padding: 24px 24px 24px 0;
  padding-left: 16px;
  overflow: auto;
}

.rol-1 .settings-menu {
  height: calc(100% - 40px);
}

.settings-menu i {
  font-style: normal;
  font-family: "fa-pro";
  font-size: 16.6px;
  width: 20px;
  height: 20px;
  display: flex;
  margin-right: 8px;
}

.settings-menu li {
  height: 40px;
  width: 100%;
  display: flex;
  align-items: center;
  padding-left: 10px;
  font-size: 14.5px;
  border-radius: 8px;
}

.settings-menu li:hover {
  background: rgba(255, 255, 255, 0.1);
}

.go-home {
  display: flex;
  height: 40px;
  width: 40px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.116);
  transition: all 100ms;
}

.go-home i {
  font-family: "fa-pro";
  font-style: normal;
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
  font-size: 16.6px;
  opacity: 0.9;
}

.go-home:hover {
  background: rgba(255, 255, 255, 0.151);
}

.go-home:active {
  transform: scale(0.9);
}
</style>
