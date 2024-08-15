<template>
  <keep-alive>
    <div>
      <expander
        :canSwap="false"
        title="更新与安全"
        description="当前版本: 0.1.0 上次检查时间: 2022/11/27 18:03"
        icon="arrows-rotate"
        :padding="[0, 0, 0, 0]">
        <setting-item title="更新通道" description="控制启动器获取更新的频率" icon="radio">
          <select-vue :options="['Weekly', 'Snapshot', 'Release']" :default="2"></select-vue>
        </setting-item>
        <setting-item title="自动更新" description="有更新可用时自动下载并安装" icon="clock">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item title="立即检查更新" icon="circle-up" :last="true">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
      </expander>
    </div>
  </keep-alive>
  <!-- <keep-alive>
    <div>
      <tip-bar style="padding:24px 34px;font-size: 14px;">
        <p style="line-height: 1.4;">Amethyst Launcher 是一个优雅、开源、可扩展的 Minecraft 启动器，由 Broken_Deer 用 ❤️ 制作，您的赞助将帮助 Amethyst Launcher 获得更好的发展</p>
        <p style="line-height: 1.4;">Amethyst Launcher 本体的所有内容都是免费的，扩展收取费用与启动器作者无关（见免责声明）</p>
      </tip-bar>
      <expander :canSwap=false title="更新与安全" description="当前版本: 0.1.0 上次检查时间: 2022/11/27 18:03" icon="arrows-rotate">
        <toggle-switch-bar name="使用预览版启动器" config="globle.update.beta"></toggle-switch-bar> 更改为「更新通道」列表选项
        <toggle-switch-bar name="自动更新" config="globle.update.auto"></toggle-switch-bar>
        <toggle-switch-bar name="检查更新时同时检查扩展更新" config="globle.update.ext"></toggle-switch-bar>
        <button-bar name="立即检查更新" text="检查更新"></button-bar>
      </expander>
    </div>
  </keep-alive> -->
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue"
import SelectVue from "@/components/controllers/Select.vue"
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue"
import Expander from "@/components/Expander.vue"
// import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
// import ButtonBar from '@/components/ButtonBar.vue';
// import TipBar from '@/components/TipBar.vue';

// import { useConfigStore } from '@/storage/configs';

// const get = useConfigStore()
// // console.log(get(""))
</script>

<style lang="less"></style>
