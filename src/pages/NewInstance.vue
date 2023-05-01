<template>
  <div style="display: flex;padding-left: 20px;">
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
import CreateNew from './newInstance/CreateNew.vue'

const components = reactive([
  {
    name: '新建配置',
    icon: 'folder-plus',
    component: markRaw(CreateNew)
  },
  // {
  //   name: '导入备份',
  //   icon: 'folders',
  //   component: markRaw(Game)
  // },
  // {
  //   name: '从其他启动器导入',
  //   icon: 'arrow-down-to-square',
  //   component: markRaw(Advance)
  // },
])
const activeComponent = shallowRef(CreateNew)
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

<style lang="less" scoped>
.content {
  padding-right: 20px;
}
</style>