<template>
  <dialog-vue :visible="show" width="460" height="480">
    <div class="main">
      <search-bar style="margin-bottom: 10px;">
      </search-bar>
      <TransitionGroup>
        <list-item v-for="(version, index) in versions" :key="index" :title="version.id" logo="1" :click-able="true" @click="$emit('select', version.id)"  :buttons="['circle-info', 'folders', 'trash-can']">
          <template #icon>
            <img v-if="version.type == `release`" style="width: 100%; height: 100%;" src="@/assets/images/minecraft.webp" alt="">
            <img v-else-if="version.type == `snapshot`" style="width: 100%; height: 100%;" src="@/assets/images/Command_Block.webp" alt="">
            <img v-else style="width: 100%; height: 100%;" src="@/assets/images/Ancient_Debris.webp" alt="">
          </template>
          <template #subtitle v-if="version.type == 'release'">
            <tag text="正式版" :color="['74', '194', '107']" :background="true"
              :border="true"></tag>
          </template>
          <template #subtitle v-else-if="version.type == 'snapshot'">
            <tag text="测试版" :color="['200', '200', '0']" :background="true" :border="true"></tag>
          </template>
          <template #subtitle v-else>
            <tag text="远古版" :color="['255', '129', '120']" :background="true" :border="true"></tag>
          </template>
        </list-item>
        <!-- <list-item v-for="(version, index) in versions" :key="index" :title="version.name" :logo="version.icon" :click-able="false"
            :buttons="['circle-info', 'folders', 'trash-can']">
            <template #subtitle v-if="version.type == 'broken'">
              <tag text="损坏" :color="['255', '129', '120']" :background="true" :border="true"></tag>
            </template>
            <template #subtitle v-else-if="mod.type == 'lib'">
              <tag text="支持库" :color="['200', '200', '0']" :border="true"></tag>
            </template>
            {{ version.description }}
          </list-item> -->
      </TransitionGroup>
    </div>
  </dialog-vue>
</template>
  
<script setup lang="ts">
import DialogVue from '@/components/Dialog.vue';
import SearchBar from '@/components/SearchBar.vue';
import { reactive, ref } from 'vue';
import { invoke } from '@tauri-apps/api';
import ListItem from '@/components/ListItem.vue';
import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
import Tag from '@/components/Tag.vue';

const props = withDefaults(defineProps<{
  show: boolean
}>(), {
  show: false
})

let versions = ref<Array<any>>([])
invoke('get_version_list').then((res: any) => {
  if (res != null) {
    versions.value = res.versions
  } else {
    throw "get_version_list failed!"
  }

}).catch((err) => {
  console.log(err)
})

</script>
  
<style lang="less" scoped>
.main {
  height: 100%;
  width: 100%;
  position: relative;
  overflow-x: visible;
  // overflow-y: ;
  margin: 12px 14px;
  width: calc(100% - 28px);
}
</style>
  