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
        <setting-item title="自定义启动器名称" description="不知道这东西有什么用" icon="signature">
          <TextInputBox width="360px" v-model="config.launch.launcher_name"></TextInputBox>
        </setting-item>
        <setting-item
          title="进程优先级"
          description="设置游戏进程的优先级，仅在 Linux 中有效"
          icon="chart-simple">
          <select-vue
            :display-name="['高', '较高', '中', '较低', '低']"
            :options="['High', 'AboveNormal', 'Normal', 'BelowNormal', 'Low']"
            :default="2"
            v-model="config.launch.process_priority"></select-vue>
        </setting-item>
        <!-- TODO:<setting-item title="服务器地址" description="启动后自动加入服务器" icon="server"> -->
        <!--   <TextInputBox -->
        <!--     width="240px" -->
        <!--     v-model="config.launch.server!.ip" -->
        <!--     style="display: inline-block; margin-right: 16px" -->
        <!--     placeholder="IP 或域名"></TextInputBox> -->
        <!--   <TextInputBox -->
        <!--     width="100px" -->
        <!--     v-model="config.launch.server!.port" -->
        <!--     placeholder="端口" -->
        <!--     style="display: inline-block"></TextInputBox> -->
        <!-- </setting-item> -->
        <setting-item title="存档名称(TODO)" description="启动后自动进入存档" icon="floppy-disk">
          <TextInputBox width="360px" placeholder="存档文件夹的名称"></TextInputBox>
        </setting-item>
        <setting-item title="全屏" description="启动游戏后自动让游戏全屏" icon="window-maximize">
          <ToggleSwitch v-model="config.launch.fullscreen"></ToggleSwitch>
        </setting-item>
        <setting-item
          :disabled="config.launch.fullscreen"
          title="窗口大小"
          description="游戏窗口的初始大小"
          icon="window">
          <TextInputBox
            width="100px"
            style="display: inline-block; margin-right: 16px"
            placeholder="宽"
            v-model.number="config.launch.width"></TextInputBox>
          <TextInputBox
            width="100px"
            style="display: inline-block"
            placeholder="高"
            v-model.number="config.launch.height">
          </TextInputBox>
        </setting-item>
        <setting-item title="启动游戏后隐藏启动器(TODO)" icon="eye-slash">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item
          title="演示模式"
          description="在单一的世界中游玩100分钟（5个游戏日），在此之后地图就会被锁定"
          icon="">
          <toggle-switch v-model="config.launch.is_demo"></toggle-switch>
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
import { useConfigStore } from "@/config";
import { ref, watch } from "vue";
const config = useConfigStore();
</script>

<style lang="less"></style>
