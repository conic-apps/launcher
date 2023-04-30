<template>
  <keep-alive>
    <div>
      <expander :expander-header=false>
        <p>Magical Launcher 是一个优雅、开源、可扩展的 Minecraft 启动器，由 Broken_Deer 用 ❤️ 制作，您的赞助将帮助 Magical Launcher 获得更好的发展</p>
        <p>Magical Launcher 的所有内容都是免费的，如果你发现有些扩展收取费用，请向我们举报</p>
      </expander>
      <expander :canSwap=false title="更新与安全" description="最新正式版 上次检查时间: 2022/11/27 18:03" icon="arrows-rotate">
        <toggle-switch-bar name="使用预览版启动器" config="globle.update.beta"></toggle-switch-bar>
        <toggle-switch-bar name="自动更新" config="globle.update.auto"></toggle-switch-bar>
        <toggle-switch-bar name="检查更新时同时检查扩展更新" config="globle.update.ext"></toggle-switch-bar>
        <toggle-switch-bar name="有重大安全更新时自动更新" config="globle.update.safe"></toggle-switch-bar>
        <button-bar name="立即检查更新" text="检查更新"></button-bar>
      </expander>
    </div>
  </keep-alive>
</template>
    
<script setup lang="ts">
import Expander from '@/components/Expander.vue';
import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
import ButtonBar from '@/components/ButtonBar.vue';

</script>
    
<style lang="less"></style>