<template>
  <div class="settings">
    <div class="rol-1">
      <ul class="settings-menu">
        <li
          @click="switchComponent(item, index)"
          :class="[activeComponentIndex == index ? 'active' : '']"
          v-for="(item, index) in components"
          :key="index">
          <i :class="item.icon"></i>{{ item.name }}
        </li>
      </ul>
      <div class="go-home" @click="$emit('back-to-home')">
        <i class="house"></i>
      </div>
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
import { markRaw, reactive, ref, shallowRef } from "vue";
import General from "./settings/General.vue";
import Game from "./settings/Game.vue";
import Advance from "./settings/Advance.vue";
import Appearance from "./settings/Appearance.vue";
import Download from "./settings/Download.vue";
import Accessibility from "./settings/Accessibility.vue";
import Extend from "./settings/Extend.vue";

const components = reactive([
  {
    name: "常规",
    icon: "house",
    component: markRaw(General),
  },
  {
    name: "游戏",
    icon: "gamepad",
    component: markRaw(Game),
  },
  {
    name: "高级",
    icon: "pro-settings",
    component: markRaw(Advance),
  },
  {
    name: "个性化",
    icon: "palette",
    component: markRaw(Appearance),
  },
  {
    name: "下载",
    icon: "download",
    component: markRaw(Download),
  },
  {
    name: "辅助功能",
    icon: "arrows-spin",
    component: markRaw(Accessibility),
  },
  {
    name: "扩展",
    icon: "cubes",
    component: markRaw(Extend),
  },
]);
const currentComponent = shallowRef(General);
let activeComponentIndex = ref(0);
let transitionName = ref("");
const content = ref<any>(null);
function switchComponent(item: any, index: number) {
  currentComponent.value = item.component;
  if (activeComponentIndex.value < index) {
    transitionName.value = "slide-up";
  } else {
    transitionName.value = "slide-down";
  }
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
  border: 1px solid #f00;
  width: 240px;
  flex-shrink: 0;
  padding: 24px;
}

.rol-2 {
  border: 1px solid #0f0;
  width: 100%;
  padding: 24px 24px 24px 0;
}

.rol-1 .settings-menu {
  border: 1px solid #00f;
  height: calc(100% - 40px);
}

.go-home {
  display: flex;
  height: 40px;
  width: 40px;
  border-radius: 50%;
  background: rgba(var(--theme-color), 0.6);
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
  background: rgba(var(--theme-color), 0.8);
}

.go-home:active {
  transform: scale(0.9);
}
</style>
