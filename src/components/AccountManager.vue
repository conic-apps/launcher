<template>
  <div class="account-manager">
    <card class="overview" id="xxxxx" title="Broken_Deer" description="UUID: xxxxxxxxxxxx" icon="" padding="10,12,10,12" margin="0,0,10,0"></card> <!-- id is uuid-->
  </div>
</template>

<script setup lang="ts">
import Card from './Card.vue';
import $ from 'jquery';

console.log($('#xxxxx .icon'))
setTimeout(() => {
  $('#xxxxx .icon').attr('style', /* css */ `background-image: url(/steve_avatar.webp); width: 36px; height: 36px; margin-right: 10px`)
}, 1);
</script>

<style lang="less" scoped>
div.account-manager .icon {
  background-position: center;
  background-size:contain;
  background-repeat: none;
}
</style>
