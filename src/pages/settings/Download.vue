<template>
  <div>
    <expander :can-swap="false" title="下载选项" description="下载源、线程限制" icon="download">
      <toggle-switch-bar name="网络节流"></toggle-switch-bar>
      <!-- todo: sliderbar 最大线程数 最大下载速度(MiB/s)-->
      <slider-bar name="最大线程数" max="256" min="1" step="1"></slider-bar>
      <slider-bar name="最大下载速度(MiB/s)" max="50" min="1" step="1"></slider-bar>
      <tip-bar style="margin-bottom: 10px;"><strong>Tip:</strong> 上面三个选项是用来限制下载速度的，如果想要获得最佳下载速度请保持默认设置</tip-bar>
      <select-bar name="下载源" :options="['自动选择 (最快最稳定)','官方源', 'BMCLAPI (MCBBS)', 'BMCLAPI (传统)']" :default="0"></select-bar>
    <text-input-bar name="自定义下载源" placeholder="填写以覆盖上个选项，自定义源必须符合条件，详见帮助文档"></text-input-bar>
    </expander>
  </div>
</template>

<script setup lang="ts">
import Expander from '@/components/Expander.vue';
import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
import { ref } from 'vue'
import SliderBar from '@/components/SliderBar.vue';
import TipBar from '@/components/TipBar.vue';
import SelectBar from '@/components/SelectBar.vue';
import TextInputBar from '@/components/TextInputBar.vue';

</script>

<style lang="less" scoped></style>