<template>
    <div>
      <div class="content">
        <expander :padding="[22, 24, 22, 24]" :expander-header="false">
          <div class="info">
            <div class="icon"></div>
            <div class="info">
              <h4>aaaaa</h4>
              <p>bbbbb</p>
            </div>
            <button class="command-button">保存配置</button>
            <text-input-bar name="名称"></text-input-bar>
          </div>
        </expander>
        <expander margin="10,0,0,0" title="Minecraft" description="aaa" icon="minecraft" :is-swaped="true"
          :padding="[16, 20, 16, 20]" :flex-start="true">
          <ul v-if="minecraftListLoaded">
            <list-item v-for="(item, index) in minecraftVersionList" :key="index" @click="chooseMinecraft()"
              :title="item.id" :click-able="true"
              :buttons="['folders', 'circle-info', 'arrow-up-right-from-square']"></list-item>
          </ul>
          <!-- todo: loading -->
        </expander>
        <expander margin="10,0,0,0" title="Forge" description="aaa" icon="forge" :is-swaped="true"
          :padding="[16, 20, 16, 20]" :flex-start="true">
          <ul v-if="minecraftListLoaded">
            <list-item v-for="(item, index) in minecraftVersionList" :key="index" @click="chooseMinecraft()"
              :title="item.id" :click-able="true"
              :buttons="['folders', 'circle-info', 'arrow-up-right-from-square']"></list-item>
          </ul>
          <!-- todo: loading -->
        </expander>
        <expander margin="10,0,0,0" title="Fabric" description="aaa" icon="fabric" :is-swaped="true"
          :padding="[16, 20, 16, 20]" :flex-start="true">
          <ul v-if="minecraftListLoaded">
            <list-item v-for="(item, index) in minecraftVersionList" :key="index" @click="chooseMinecraft()"
              :title="item.id" :click-able="true"
              :buttons="['folders', 'circle-info', 'arrow-up-right-from-square']"></list-item>
          </ul>
          <!-- todo: loading -->
        </expander>
        <expander margin="10,0,0,0" title="Quilt" description="aaa" icon="quilt" :is-swaped="true"
          :padding="[16, 20, 16, 20]" :flex-start="true">
          <ul v-if="minecraftListLoaded">
            <list-item v-for="(item, index) in minecraftVersionList" :key="index" @click="chooseMinecraft()"
              :title="item.id" :click-able="true"
              :buttons="['folders', 'circle-info', 'arrow-up-right-from-square']"></list-item>
          </ul>
          <!-- todo: loading -->
        </expander>
      </div>
    </div>
  </template>
  
  <script setup lang="ts">
  import { markRaw, ref, shallowRef, type Ref } from 'vue'
  import Expander from '@/components/Expander.vue';
  import TextInputBar from '@/components/TextInputBar.vue';
  import ListItem from '@/components/ListItem.vue';
  
  let minecraftListLoaded = ref(false)
  let minecraftVersionList: Ref<any> = ref([])
  //todo: get version list & choose version
  function chooseMinecraft() {
  
  }
  </script>
  
  <style lang="less" scoped></style>