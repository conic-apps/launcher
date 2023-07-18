<template>
  <div>
    <expander :can-swap="false" title="辅助功能" description="使你的启动器更易于使用" icon="hand">
      <toggle-switch-bar name="正式版更新提示"></toggle-switch-bar>
      <toggle-switch-bar name="快照版更新提示"></toggle-switch-bar>
      <toggle-switch-bar name="默认设置为系统语言"></toggle-switch-bar>
      <select-bar name="动画速度" :options="['特别慢', '慢', '正常', '快', '特别快']" :default="2"></select-bar>
    </expander>
    <expander :can-swap="false" title="无障碍" description="无障碍功能优化" icon="wheelchair">
      <toggle-switch-bar name="禁用所有动画"></toggle-switch-bar>
      <!-- <toggle-switch-bar name="为屏幕阅读器优化"></toggle-switch-bar> -->
      <!-- <toggle-switch-bar name="高对比度模式"></toggle-switch-bar> 高对比度主题 -->
    </expander>
    <expander :can-swap="false" title="快捷键" description="设置各项常用操作的快捷键" icon="keyboard">

    </expander>
    <expander :can-swap="false" title="默认搜索过滤器" description="指定各搜索框中的默认过滤参数，以便过滤掉不常用的项" icon="filter">

    </expander>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Expander from '@/components/Expander.vue';
import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
import SelectBar from '@/components/SelectBar.vue';
</script>

<style lang="less" scoped></style>