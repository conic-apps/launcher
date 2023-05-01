<template>
  <div>
    <expander :expander-header="false">
      如果没有特殊需要，请使用默认设置以获得最佳下载速度
    </expander>
    <expander :can-swap="false" title="下载选项" description="下载源、线程限制" icon="download">
      <toggle-switch-bar name="网络节流"></toggle-switch-bar>
      <!-- todo: sliderbar 最大线程数 最大下载速度(MiB/s)-->
      <slider-bar name="最大线程数" max="256" min="1" step="1"></slider-bar>
      <slider-bar name="最大下载速度(MiB/s)" max="50" min="1" step="1"></slider-bar>
    </expander>
  </div>
</template>

<script setup lang="ts">
import Expander from '@/components/Expander.vue';
import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
import { ref } from 'vue'
import SliderBar from '@/components/SliderBar.vue';

</script>

<style lang="less" scoped></style>