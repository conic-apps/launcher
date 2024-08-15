<template>
  <div class="settings" style="display: flex; padding-left: 10px; border-radius: 0px">
    <div class="sidebar">
      <ul>
        <li
          @click="switchComponent(item, index)"
          :class="[activeComponentIndex == index ? 'active' : '']"
          v-for="(item, index) in components"
          :key="index">
          <i :class="item.icon"></i>{{ item.name }}
        </li>
      </ul>
      <li @click="$emit('backToHome')" style="margin-bottom: -9px" class="backtoHome">
        <i class="arrow-left"></i>返回
      </li>
    </div>
    <div class="content" ref="content">
      <Transition :name="transitionName" mode="out-in">
        <KeepAlive>
          <component :is="activeComponent"></component>
        </KeepAlive>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, shallowRef, type Ref, markRaw } from "vue"
import $ from "jquery"
import General from "./settings/General.vue"
import Game from "./settings/Game.vue"
import Advance from "./settings/Advance.vue"
import Appearance from "./settings/Appearance.vue"
import Download from "./settings/Download.vue"
import Accessibility from "./settings/Accessibility.vue"
import Extend from "./settings/Extend.vue"

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
])
const activeComponent = shallowRef(General)
let activeComponentIndex = ref(0)
let transitionName = ref("")
const content = ref<any>(null)
function switchComponent(item: any, index: number) {
  activeComponent.value = item.component
  if (activeComponentIndex.value < index) {
    transitionName.value = "slide-up"
  } else {
    transitionName.value = "slide-down"
  }
  activeComponentIndex.value = index
}
</script>

<style lang="less">
.content {
  width: 100%;
  padding-right: 30px;
  padding-top: 10px;
  overflow-y: overlay;
  overflow-x: visible;
}

.content > div {
  width: 100%;
}

.settings {
  padding-left: 20px !important;
  margin-left: -60px;
}

.sidebar {
  width: 220px;
  font-size: 16px;
  margin-bottom: 20px;
  margin-right: 20px;
  transition: all 0.2s cubic-bezier(0, 0.83, 0.47, 1);
  padding: 26px 0px 0 0px;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  justify-content: space-between;
}

.sidebar ul {
  padding-left: 4px;
  padding-bottom: 4px;
  padding-right: 4px;
  list-style-type: none;
  display: flex;
  width: 100%;
  flex-wrap: wrap;
  max-height: calc(100vh - 196px);
  overflow: visible overlay;
}

.sidebar li {
  list-style: none;
  font-size: 13.5px;
  font-weight: 500;
  line-height: 1.3;
  width: 100%;
  margin: 0.1rem 0;
  border-radius: var(--border-radius-small);
  padding: 7px 7px;
  display: flex;
  align-items: center;
  transition: all 20ms ease;
  color: #000000cd;
}

.sidebar li:hover {
  background-color: #ffffff45;
}

.sidebar li:active {
  opacity: 0.55;
}

.sidebar .active {
  background: rgba(var(--theme-color), 0.09);
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.1);
  pointer-events: none;
  color: rgba(var(--theme-color), 0.9);
}

.sidebar li::before {
  background-color: #007bff00;
  content: "";
  height: 14px;
  width: 4px;
  position: relative;
  left: -7px;
  border-radius: 5px;
  transition: all 0.15s ease;
}

.sidebar .active::before {
  background-color: rgba(var(--theme-color), 1);
  content: "";
  height: 18px;
}

.sidebar i {
  font-family: "fa-pro";
  border-radius: 100px;
  font-style: normal;
  font-size: 16px;
  font-weight: 400;
  margin-right: 7px;
  width: 21px;
  display: inline-flex;
  justify-content: center;
  align-items: center;
  transition: all 80ms ease;
}

.sidebar > div {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-direction: row;
}

.sidebar > div > p {
  font-size: 14px;
  margin: 10px 2px;
  display: inline;
  width: fit-content;
}
</style>
