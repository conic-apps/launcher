<template>
  <div>
    <expander :padding="[22, 24, 22, 24]" :expander-header="false">
      <div class="info-bar">
        <div class="icon"></div>
        <div class="info">
          <h4>aaaaa</h4>
          <p>bbbbb</p>
        </div>
        <button class="command-button">保存配置</button>
      </div>
      <text-input-bar name="名称"></text-input-bar>
    </expander>
    <div style="display: flex;">
      <div style="width: 100%;margin-right: 4px;">
        <card-link margin="0,0,8,0" title="Minecraft" description="选择Minecraft版本" icon="minecraft"></card-link>
        <card-link margin="0,0,0,0" title="Forge" description="请先选择Minecraft版本" icon="forge"></card-link>
      </div>
      <div style="width: 100%;margin-left: 4px;">
        <card-link margin="0,0,8,0" title="Fabric" description="请先选择Minecraft版本" icon="fabric"></card-link>
        <card-link margin="0,0,0,0" title="Quilt" description="请先选择Minecraft版本" icon="quilt"></card-link>
      </div>
    </div>
  </div>
</template>
  
<script setup lang="ts">
import { markRaw, ref, shallowRef, type Ref } from 'vue'
import Expander from '@/components/Expander.vue';
import TextInputBar from '@/components/TextInputBar.vue';
import ListItem from '@/components/ListItem.vue';
import CardLink from '@/components/CardLink.vue';

let minecraftListLoaded = ref(false)
let minecraftVersionList: Ref<any> = ref([])
//todo: get version list & choose version
function chooseMinecraft() {

}
</script>
  
<style lang="less" scoped>
.info-bar {
  display: flex;
  margin-bottom: 12px;
  height: 40px;
  align-items: center;
}

.info-bar .icon {
  background-image: url(@/assets/images/Ancient_Debris.webp);
  background-size: cover;
  width: 36px;
  height: 36px;
  margin-right: 8px;
}
</style>