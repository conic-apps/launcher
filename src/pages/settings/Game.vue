<template>
  <keep-alive>
    <div>
      <setting-group title="Java 虚拟机设置">
        <setting-item title="选择 Java" icon="java" :clickAble="true">
          <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">已选择 Java 17</span>
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
        <setting-item title="安装或导入 Java" description="" icon="download" :clickAble="true">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
        <!-- <setting-item title="Java 内存"></setting-item> -->
      </setting-group>
      <setting-group title="启动选项">
        <setting-item title="游戏窗口标题" description="自定义游戏窗口标题" icon="heading">
          <TextInputBox width="360px"></TextInputBox>
        </setting-item>
        <setting-item
          title="自定义启动器名称"
          description="启动器名称将显示在游戏主菜单下方"
          icon="signature">
          <TextInputBox width="360px"></TextInputBox>
        </setting-item>
        <setting-item
          title="进程优先级"
          description="设置游戏进程的优先级，仅在 Linux 中有效"
          icon="chart-simple">
          <select-vue :options="['最高', '较高', '常规', '较低', '最低']" :default="2"></select-vue>
        </setting-item>
        <setting-item title="服务器地址" description="启动后自动加入服务器" icon="server">
          <TextInputBox width="360px"></TextInputBox>
        </setting-item>
        <setting-item title="存档名称" description="启动后自动进入存档" icon="floppy-disk">
          <TextInputBox width="360px"></TextInputBox>
        </setting-item>
        <setting-item title="窗口大小" description="游戏窗口的初始大小" icon="window">
          <TextInputBox
            width="100px"
            style="display: inline-block; margin-right: 16px"
            placeholder="宽"></TextInputBox>
          <TextInputBox width="100px" style="display: inline-block" placeholder="高"></TextInputBox>
        </setting-item>
        <setting-item title="启动游戏后隐藏启动器" icon="eye-slash">
          <toggle-switch></toggle-switch>
        </setting-item>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SelectVue from "@/components/controllers/Select.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
</script>

<style lang="less"></style>
