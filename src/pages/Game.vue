<template>
  <keep-alive>
    <div class="game-page-main">
      <div class="row-1">
        <instance-info minecraft-version="1.20.1" :instance-name="currentInstance" :installed="true"
          game-button-type="launch"></instance-info>
        <assets-manager :instance-name="currentInstance" style="margin-top: 20px;"></assets-manager>
      </div>
      <div class="row-2">
        <div class="group-name"> <!--todo: move to a component-->
          <div style="display: flex; justify-content: space-between; align-items: center; height: 100%;">
            <p style="margin-left: 4px;">帐户</p>
            <button class="group-button" style="margin-right: 6px;">查看全部 <i class="chevron-right" style="font-size: 12px;"></i></button>
          </div>
        </div>
        <account-manager></account-manager>
        <div class="group-name">
          <div style="display: flex; justify-content: space-between; align-items: center; height: 100%;">
            <p style="margin-left: 4px;"></p>
            <button class="group-button" style="margin-right: 6px;">查看全部 <i class="chevron-right" style="font-size: 12px;"></i></button>
          </div>
        </div>
        <instance-manager></instance-manager>
      </div>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import InstanceInfo from '@/components/InstanceInfo.vue';
import AssetsManager from '@/components/AssetsManager.vue';
import AccountManager from '@/components/AccountManager.vue';
import InstanceManager from '@/components/InstanceManager.vue';

import { ref } from 'vue';

let currentInstance = ref("1.20")
</script>

<style lang="less" scoped>
.game-page-main {
  width: 100%;
  height: 100%;
  display: flex;
}

.row-1,
.row-2 {
  height: 100%;
  display: flex;
  flex-direction: column;
  // border: 1px solid #fff;
  padding: 24px 24px;
}

.row-1 {
  width: 100%;
}

.row-2 {
  width: 328px;
  flex-shrink: 0;
}

// todo: move to main.css
.group-name {
  width: 100%;
  height: 32px;
  font-size: 15.5px;
  margin-bottom: 8px;
}

.group-button {
  background: rgba(255, 255, 255, 0.08);
  border: none;
  border-radius: 4px;
  padding: 2px 8px;
  font-size: 12px;
  cursor: pointer;
}
</style>