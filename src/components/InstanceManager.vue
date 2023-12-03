<template>
  <div class="instance-manager">
    <card id="xx" title="最新版本" description="Minecraft 1.20.3" icon="grass-block" padding="10,12,10,12" margin="0,0,10,0"
      icon-size="36,36" :icon-background="false">
    </card>
    <card id="xx" title="最新快照" description="Minecraft 1.20.3" icon="command-block" padding="10,12,10,12" icon-size="36,36"
      margin="0,0,10,0" :icon-background="false">
    </card> <!-- id is instance name-->
  </div>
</template>
  
<script setup lang="ts">
import Card from './Card.vue';
import $ from 'jquery';


</script>
  
<style lang="less" scoped>
div.instance-manager .icon {
  background-position: center;
  background-size: contain;
  background-repeat: none;
}
</style>
  