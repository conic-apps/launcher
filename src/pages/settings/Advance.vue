<template>
  <!-- <KeepAlive>
    <div>
      <tip-bar  type="danger" style="margin-bottom: 8px;font-size: 14px;">如果你不清楚这些选项的作用，请不要乱动它们</tip-bar>
      <tip-bar style="font-size: 14px;">你可以使用右上角的重置按钮将此页设置恢复为默认值</tip-bar>
      <expander :canSwap=false title="高级设置" description="自定义游戏启动行为" icon="pro-settings">
        <toggle-switch-bar name="检查游戏库文件完整性" config="globle.advanced.checklib"></toggle-switch-bar>
        <select-bar name="Java 垃圾回收器" config=""
          :options="['默认(自动选择)', 'SerialGC', 'ParallelGC', 'G1GC', 'ZGC']"
          :default="0"></select-bar>
      </expander>
      <expander :canSwap=false title="调试选项" description="这些设置只用于调试" icon="debug-settings">
        <toggle-switch-bar name="将启动器日志保存到文件" config="globle.advanced.savelog"></toggle-switch-bar>
        <button-bar name="导出启动器日志" text="导出"></button-bar>
        <button-bar name="打开开发人员工具" text="启动 DevTools" @click="openDevtools"></button-bar>
      </expander>
    </div>
  </KeepAlive> -->
</template>

<script setup lang="ts">
// import { ref } from 'vue'
// import ButtonBar from '@/components/ButtonBar.vue';
// import Expander from '@/components/Expander.vue';
// import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
// import SelectBar from '@/components/SelectBar.vue';
// import TipBar from '@/components/TipBar.vue';

// function openDevtools() {

// }
// function updateData(a: any, b: any) {

// }
</script>

<style lang="less" scoped></style>