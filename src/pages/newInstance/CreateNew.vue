<template>
  <div>
    <expander :padding="[22, 24, 22, 24]" :expander-header="false">
      <div class="info-bar">
        <div class="icon"></div>
        <div class="info">
          <h4>{{ instanceName }}</h4>
          <p>
            <span v-if="select.minecraft">Minecraft {{ select.minecraft }}</span>
            <span v-if="select.forge">, Forge {{ select.forge }}</span>
            <span v-if="select.fabric">, Fabric {{ select.fabric }}</span>
            <span v-if="select.quilt">, Quilt {{ select.quilt }}</span>
          </p>
        </div>
        <button class="command-button">保存配置</button>
      </div>
      <text-input-bar name="实例名称"></text-input-bar>
    </expander>
    <div style="display: flex;">
      <div style="width: 100%;margin-right: 4px;">
        <card-link margin="0,0,8,0" title="Minecraft" description="选择Minecraft版本" icon="minecraft"
          @click="showMinecraft = true"></card-link>
        <minecraft-choose :show="showMinecraft" @select="setMinecraft"></minecraft-choose>
        <card-link :class="select.minecraft ? '' : 'disabled'" margin="0,0,0,0" title="Forge" description="请先选择Minecraft版本" icon="forge"></card-link>
      </div>
      <div style="width: 100%;margin-left: 4px;">
        <card-link :class="select.minecraft ? '' : 'disabled'" margin="0,0,8,0" title="Fabric" description="请先选择Minecraft版本" icon="fabric"></card-link>
        <card-link :class="select.minecraft ? '' : 'disabled'" margin="0,0,0,0" title="Quilt" description="请先选择Minecraft版本" icon="quilt"></card-link>
      </div>
    </div>
  </div>
</template>
  
<script setup lang="ts">
import { markRaw, ref, shallowRef, type Ref, reactive, computed } from 'vue'
import Expander from '@/components/Expander.vue';
import TextInputBar from '@/components/TextInputBar.vue';
import ListItem from '@/components/ListItem.vue';
import CardLink from '@/components/CardLink.vue';
import MinecraftChoose from '../dialogs/MinecraftChoose.vue';

let showMinecraft = ref(false)
let showForge = ref(false)
let showFabric = ref(false)
let showQuilt = ref(false)

let select = reactive({
  minecraft: '',
  forge: '123',
  fabric: '',
  quilt: '',
})

let instanceName = computed(() => {
  return `${select.minecraft}${select.forge ? '-forge ' + select.forge : ''}${select.fabric ? '-fabric ' + select.fabric : ''}${select.quilt ? '-quilt ' + select.quilt : ''}`
})

//todo: get version list & choose version
function setMinecraft(versionId: string) {
  select.minecraft = versionId
  showMinecraft.value = false
}
function setForge() {

}
function setFabric() {

}
function setQuilt() {

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

.info h4 {
  font-weight: 500;
  font-size: 15px;
  // color: rgb(var(--theme-color));
  line-height: 1;
}

.info p {
  opacity: 0.6;
}

.disabled {
  opacity: 0.5;
  pointer-events: none;
}
</style>