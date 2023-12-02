<template>
  <keep-alive>
    <div class="game-page-main">
      <div class="row-1">
        <instance-info minecraft-version="1.20.1" :instance-name="currentInstance" :installed="true"
          game-button-type="launch"></instance-info>
        <assets-manager :instance-name="currentInstance" style="margin-top: 20px;"></assets-manager>
      </div>
      <div class="row-2">
        <div class="group-name">
          帐户
        </div>
      </div>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import InstanceInfo from '@/components/InstanceInfo.vue';
import AssetsManager from '@/components/AssetsManager.vue';
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

.group-name {
  width: 100%;
  height: 32px;
  font-size: 16px;

}</style>