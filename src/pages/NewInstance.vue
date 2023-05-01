<template>
  <div class="settings" style="display: flex;padding-left: 10px;">
    <div class="sidebar">
      <ul>
        <li @click="switchComponent(item, index)" :class="[activeComponentIndex == index ? 'active' : '']"
          v-for="(item, index) in components" :key="index"><i :class="item.icon"></i>{{ item.name }}</li>
      </ul>
      <li @click="$emit('backToHome')" style="margin-bottom: -9px;" class="backtoHome"><i class="arrow-left"></i>返回</li>
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
import { markRaw, reactive, ref, shallowRef } from 'vue'
const components = reactive([
  {
    name: '新建配置',
    icon: 'house',
    component: markRaw(General)
  },
  {
    name: '导入备份',
    icon: 'gamepad',
    component: markRaw(Game)
  },
  {
    name: '从其他启动器导入',
    icon: 'pro-settings',
    component: markRaw(Advance)
  },
  {
    name: ''
  }
])
const activeComponent = shallowRef(General)
let activeComponentIndex = ref(0)
let transitionName = ref('')
const content = ref<any>(null)
function switchComponent(item: any, index: number) {
  activeComponent.value = item.component
  if (activeComponentIndex.value < index) {
    transitionName.value = 'slide-up'
  } else {
    transitionName.value = 'slide-down'
  }
  activeComponentIndex.value = index
}
</script>

<style lang="less" scoped></style>