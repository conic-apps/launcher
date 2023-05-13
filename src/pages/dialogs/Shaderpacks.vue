<template>
  <dialog-vue :visible="show" width="460" height="480">
    <div style="position: relative;margin: 12px 14px; width: calc(100% - 28px);">
      <div
        style="display: flex; justify-content: space-between; border-bottom: 2px solid rgba(var(--theme-color), 0.6);margin-bottom: 10px;">
        <div class="info">
          <div class="icon"></div>
          <div class="text">
            <h4 class="name"><span>{{ instanceName }}</span>中的光影包</h4>
            <p>共安装有 {{ shaderpacks.length }} 个光影包</p>
          </div>
        </div>
        <div class="buttons"><dialog-button icon="close" @click="$emit('close')"></dialog-button></div>
      </div>
      <search-bar
        style="margin-bottom: 8px; position: sticky; top: 0; right: 0; bottom: 0; left: 0; z-index: 1000; background: #fff; border: 1px solid #00000028; box-shadow: 0 0 10px #00000012;"></search-bar>
      <TransitionGroup>
        <list-item v-for="(shadeerpack, index) in shaderpacks" :key="index" :title="shadeerpack" :click-able="false"
          :buttons="['circle-info', 'folders', 'trash-can']">
        </list-item>
      </TransitionGroup>
    </div>
  </dialog-vue>
</template>
    
<script setup lang="ts">
import { reactive, ref } from 'vue'
import DialogVue from '@/components/Dialog.vue';
import ListItem from '@/components/ListItem.vue';
import Tag from '@/components/Tag.vue';
import SearchBar from '@/components/SearchBar.vue';
import DialogButton from '@/components/DialogButton.vue';

const props = withDefaults(defineProps<{
  show: boolean,
  shaderpacks?: any,
  instanceName: string
}>(), {
  shaderpacks: () => {
    return [
      'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa', 'aaa',
    ]
  }
})
</script>
    
<style lang="less" scoped>
.info {
  display: flex;
  align-items: center;
  padding-bottom: 6px;
}

.info .icon {
  width: 40px;
  height: 40px;
  background: url(@/assets/images/Ancient_Debris.webp);
  background-position: center;
  background-size: cover;
}

.info .text {
  margin-left: 6px;
}

.info h4 {
  font-weight: 400;
  font-size: 18px;
  margin-bottom: 2px;
}

.info span {
  color: rgb(var(--theme-color));
}

.info p {
  color: #000000a0;
  font-size: 13px;
}
</style>
    