<template>
  <dialog-vue :visible="show" width="460" height="480">
    <div style="position: relative;margin: 12px 14px; width: calc(100% - 28px);">
      <div style="display: flex; justify-content: space-between; border-bottom: 2px solid rgba(var(--theme-color), 0.6);margin-bottom: 10px;">
        <div class="info">
          <div class="icon"></div>
          <div class="text">
            <h4 class="name"><span>{{ instanceName }}</span>中的存档</h4>
            <p>共安装有 {{ worlds.length }} 个存档</p>
          </div>
        </div>
        <div class="buttons"><dialog-button icon="close" @click="$emit('close')"></dialog-button></div>
      </div>
      <search-bar style="margin-bottom: 8px; position: sticky;"></search-bar>
      <TransitionGroup>
        <list-item v-for="(world, index) in worlds" :key="index" :title="world.name" :logo="world.icon" :click-able="true"
          :buttons="['circle-info', 'folders', 'trash-can']">
          <template #subtitle v-if="world.type == 'broken'">
            <tag text="损坏" :color="['255', '129', '120']" style="transform: scale(0.9);" :background="true"></tag>
          </template>
          {{ world.description }}
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

const props = withDefaults(defineProps<{
  show: boolean,
  worlds?: any,
  instanceName: string
}>(), {
  worlds: () => {
    return [
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg',
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg',
        type: 'broken'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      }, {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg',
        type: 'lib'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg',
        type: 'broken'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      },
      {
        name: 'a',
        description: 'bbbbbbb',
        icon: '/test.svg'
      }
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
  