<template>
  <div>
    <expander :canSwap=false title="主题" description="更改主题、加载自定义主题" icon="brush">
      <button-bar name="更改主题" text="更改"></button-bar>
      <button-bar name="安装新主题" text="转到「市场」"></button-bar>
      <button-bar name="打开自定义主题文件夹" text="打开"></button-bar>
      <button-bar name="主题色" text="更改" @click="openColorChooser"></button-bar>
    </expander>
    <expander :canSwap=false title="背景" description="" icon="window-flip">
      
  </expander>
</div></template>

<script setup lang="ts">
import { ref } from 'vue'
import ButtonBar from '@/components/ButtonBar.vue';
import Expander from '@/components/Expander.vue';
import CardLink from '@/components/CardLink.vue';
function openColorChooser() {

}
</script>

<style lang="less" scoped></style>