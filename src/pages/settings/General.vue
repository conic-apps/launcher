<template>
  <keep-alive>
    <div>
      <setting-group>
        <setting-item title="语言" icon="language">
          <select-vue :options="['English', '简体中文']" :default="1"></select-vue>
        </setting-item>
        <setting-item title="更新通道" description="控制启动器获取更新的频率" icon="radio">
          <select-vue :options="['Weekly', 'Snapshot', 'Release']" :default="2"></select-vue>
        </setting-item>
        <setting-item title="自动更新" description="有更新可用时自动下载并安装" icon="clock">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item title="立即检查更新" icon="circle-up" :clickAble="true">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SelectVue from "@/components/controllers/Select.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
import SettingGroup from "@/components/SettingGroup.vue";
</script>

<style lang="less"></style>
