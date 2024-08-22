<template>
  <keep-alive>
    <div>
      <setting-group title="辅助功能">
        <setting-item
          title="正式版更新提示"
          description="当 Miencraft 正式版发布时弹窗提示"
          icon="circle-exclamation">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item
          title="快照版更新提示"
          description="当 Miencraft 快照版发布时弹窗提示"
          icon="circle-exclamation">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item
          title='隐藏"最新版本"'
          description='不在游戏列表中显示"最新版本"'
          icon="eye-slash"
          :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item
          title='隐藏"最新快照"'
          description='不在游戏列表中显示"最新快照"'
          icon="eye-slash"
          :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item
          title="自动更改游戏语言"
          description="在首次启动游戏时将游戏语言设置为你的启动器语言"
          icon="language"
          :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
      </setting-group>
      <setting-group title="无障碍">
        <setting-item
          title="禁用所有动画"
          description="眨眼和闪烁的动画对于有认知问题的人来说是有问题的，比如注意力缺陷多动障碍 (ADHD)。此外，某些动画效果可以触发前庭神经紊乱、癫痫、偏头痛和暗点敏感性。"
          icon="pause">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item
          title="高对比度模式"
          description="启用当前主题的高对比度模式（如果可用）"
          icon="laptop"
          :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
</script>

<style lang="less" scoped></style>
