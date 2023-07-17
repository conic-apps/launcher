<template>
  <dialog-vue :visible="show" width="460" height="480">
    <div style="position: relative;margin: 12px 14px; width: calc(100% - 28px);">
      <div
        style="display: flex; justify-content: space-between; border-bottom: 2px solid rgba(var(--theme-color), 0.6);margin-bottom: 10px;">
        <div class="info">
          <div class="icon"></div>
          <div class="text">
            <h4 class="name"><span>{{ instanceName }}</span>中的存档</h4>
            <p>共有 {{ datas.length }} 个存档</p>
          </div>
        </div>
        <div class="buttons"><dialog-button icon="close" @click="$emit('close')"></dialog-button></div>
      </div>
      <search-bar
        style="margin-bottom: 8px; position: sticky; top: 0; right: 0; bottom: 0; left: 0; z-index: 1000; background: #fff; border: 1px solid #00000028; box-shadow: 0 0 10px #00000012;"></search-bar>
      <TransitionGroup>
        <list-item v-for="(world, index) in datas" :key="index" :title="world.level_data.LevelName" :logo="world.icon"
          :click-able="false" :buttons="['circle-info', 'folders', 'arrow-up-right-from-square']">
          <template #subtitle>
            <tag :text="world.level_data.Version.Name" :color="['180', '180', '180']" text-color="#00000080" :border="true"></tag>
            <tag v-if="world.level_data.allowCommands" text="允许作弊" :color="['180', '180', '180']" text-color="#00000080" :border="true"></tag>
            <tag v-if="world.level_data.hardcore" text="极限模式" :color="['180', '180', '180']" text-color="#00000080" :border="true"></tag>
          </template>
          {{ world.dir_name }}
        </list-item>
      </TransitionGroup>
    </div>
  </dialog-vue>
</template>
  
<script setup lang="ts">
import DialogVue from '@/components/Dialog.vue';
import ListItem from '@/components/ListItem.vue';
import Tag from '@/components/Tag.vue';
import SearchBar from '@/components/SearchBar.vue';
import DialogButton from '@/components/DialogButton.vue';

const props = defineProps<{
  show: boolean,
  worlds?: any,
  instanceName: string,
  datas: any[],
}>()

function gameType(type: number): string {
  switch (type) {
    case 0:
      return '生存模式'
    case 1:
      return '创造模式'
    case 2: 
      return '冒险模式'
    case 3:
      return '旁观模式'
    default:
      return ''
  }
}
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
  font-size: calc(18px - var(--font-size-error));
  margin-bottom: 2px;
}

.info span {
  color: rgb(var(--theme-color));
}

.info p {
  color: #000000a0;
  font-size: calc(13px - var(--font-size-error));
}
</style>
