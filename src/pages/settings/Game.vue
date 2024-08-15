<template>
  <keep-alive>
    <div>
      <expander
        :canSwap="false"
        title="Java 虚拟机"
        description="设置游戏运行所需 Java 的位置或安装 Java"
        icon="java-logo fa-brands">
        <setting-item title="自动选择合适的 Java"></setting-item>
        <button-bar name="手动选择 Java 虚拟机" text="查找已安装的 Java"></button-bar>
        todo: input file
        <card-link
          margin="0,0,6,-4"
          title="安装 Java 17"
          description="将 Java 17 安装在当前游戏目录下"
          icon="java-logo fa-brands"
          :box-shadow="false"
          padding="8,10,8,10"></card-link>
        <card-link
          margin="0,0,6,-4"
          title="安装 Java 11"
          description="将 Java 11 安装在当前游戏目录下"
          icon="java-logo fa-brands"
          :box-shadow="false"
          padding="8,10,8,10"></card-link>
        <card-link
          margin="0,0,0,-4"
          title="安装 Java 8"
          description="将 Java 8 安装在当前游戏目录下"
          icon="java-logo fa-brands"
          :box-shadow="false"
          padding="8,10,8,10"></card-link>
      </expander>
    </div>
  </keep-alive>
  <!-- <div>
    <expander :canSwap=false title="Java 虚拟机" description="设置游戏运行所需 Java 的位置或安装 Java" icon="java-logo fa-brands">
      <toggle-switch-bar name="自动选择合适的 Java"></toggle-switch-bar>
      <button-bar name="手动选择 Java 虚拟机" text="查找已安装的 Java"></button-bar>
      todo: input file
      <card-link margin="0,0,6,-4" title="安装 Java 17" description="将 Java 17 安装在当前游戏目录下" icon="java-logo fa-brands"
        :box-shadow="false" padding="8,10,8,10"></card-link>
      <card-link margin="0,0,6,-4" title="安装 Java 11" description="将 Java 11 安装在当前游戏目录下" icon="java-logo fa-brands"
        :box-shadow="false" padding="8,10,8,10"></card-link>
      <card-link margin="0,0,0,-4" title="安装 Java 8" description="将 Java 8 安装在当前游戏目录下" icon="java-logo fa-brands"
        :box-shadow="false" padding="8,10,8,10"></card-link>
    </expander>
    <expander :canSwap=false title="启动选项" description="自定义游戏启动行为" icon="start-game">
      <text-input-bar name="游戏窗口标题" placeholder="默认"></text-input-bar>
      <text-input-bar name="自定义启动器名称" placeholder="默认"></text-input-bar>
      <text-input-bar name="服务器地址" placeholder="启动游戏后将自动进入此服务器"></text-input-bar>
      <select-bar name="进程优先级" :options="['最高', '较高', '中', '较低', '最低']" :default="0"></select-bar>
      <div style="display: flex;justify-content: space-between;margin-bottom: 10px;">
        <span class="name">窗口大小</span>
        <div style="display: flex;align-items: center;">
          <text-input-box name="宽" placeholder="800" style="margin-right: 14px;"></text-input-box>
          x
          <text-input-box name="高" placeholder="600" style="margin-left: 14px;"></text-input-box>
        </div>
      </div>
      <toggle-switch-bar name="自动分配内存"></toggle-switch-bar>
    </expander>
    <expander title="高级设置" description="自定义启动命令" icon="pro-settings" :is-swaped=true>
      <text-input-bar name="游戏参数" placeholder="填写此字段以添加至默认设置后	"></text-input-bar>
      <text-input-bar name="启动前执行" placeholder="在游戏启动前执行"></text-input-bar>
      <text-input-bar name="包装命令" placeholder="会自动添加至启动命令前方"></text-input-bar>
      <text-input-bar name="启动后执行" placeholder="在游戏启动后执行"></text-input-bar>
      <text-input-bar name="Java 虚拟机参数" placeholder="填写此字段以覆盖默认设置"></text-input-bar>
      <text-input-bar name="内存永久保存区域" placeholder="填写整数（单位 MB）"></text-input-bar>
    </expander>
    <expander title="调试选项" description="如果你不知道这些选项的作用，请不要乱动它们！" icon="debug-settings" :is-swaped=true>
      <text-input-bar name="本地库路径 (LWJGL)" placeholder="默认由启动器提供，填写此选项以自定义游戏需要的本地库"></text-input-bar>
      <toggle-switch-bar name="不检查默认的 JVM 参数"></toggle-switch-bar>
      <toggle-switch-bar name="不检查资源文件完整性"></toggle-switch-bar>
      <toggle-switch-bar name="不检查 JVM 与游戏的兼容性"></toggle-switch-bar>
      <toggle-switch-bar name="[Linux] 使用系统GLFW"></toggle-switch-bar>
      <toggle-switch-bar name="[Linux] 使用系统OpenAL"></toggle-switch-bar>
    </expander>
  </div> -->
</template>

<script setup lang="ts">
import Expander from "@/components/Expander.vue";
import SettingItem from "@/components/SettingItem.vue";
import InstanceManager from "@/pages/dialogs/InstanceManager.vue";
// import { ref } from 'vue'
// import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';
// import TextInputBar from '@/components/TextInputBar.vue';
// import TextInputBox from '@/components/TextInputBox.vue';
// import SelectBar from '@/components/SelectBar.vue';
// import CardLink from '@/components/CardLink.vue';
// import ButtonBar from '@/components/ButtonBar.vue';
</script>

<style lang="less" scoped></style>
