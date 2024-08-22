<template>
  <keep-alive>
    <div>
      <setting-group>
        <setting-item
          title="导出启动器日志(TODO)"
          description="将启动器运行日志保存到文件，以便报告问题时使用"
          :clickAble="true"
          icon="scroll">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
        <setting-item title="打开开发人员工具(TODO)" icon="code-simple" :clickAble="true">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
      </setting-group>
      <setting-group title="启动参数">
        <setting-item title="Java 垃圾回收器">
          <select-vue
            :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
            :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
            v-model="config.launch.gc"
            :default="0"></select-vue>
        </setting-item>
        <setting-item title="添加 JVM 参数" description="添加 JVM 参数">
          <TextInputBox width="360px" v-model="config.launch.extra_jvm_args"></TextInputBox>
        </setting-item>
        <setting-item title="添加游戏参数" description="添加游戏需要的其他参数">
          <TextInputBox width="360px" v-model="config.launch.extra_mc_args"></TextInputBox>
        </setting-item>
        <setting-item title="添加类路径" description="Windows下用分号隔开，其他系统用冒号">
          <TextInputBox width="360px" v-model="config.launch.extra_class_paths"></TextInputBox>
        </setting-item>
        <setting-item title="启动前执行" description="将会添加至启动命令的前一行">
          <TextInputBox width="360px" v-model="config.launch.execute_before_launch"></TextInputBox>
        </setting-item>
        <setting-item title="包装命令" description="将会添加至启动命令的开头">
          <TextInputBox width="360px" v-model="config.launch.wrap_command"></TextInputBox>
        </setting-item>
        <setting-item title="启动后执行" description="将会添加至启动脚本的最后一行">
          <TextInputBox width="360px" v-model="config.launch.execute_after_launch"></TextInputBox>
        </setting-item>
        <setting-item
          title="忽略无效的 Minecraft 凭证"
          description="将 <code>-Dfml.ignoreInvalidMinecraftCertificates=true</code> 添加到 JVM 参数中">
          <ToggleSwitch
            v-model="config.launch.ignore_invalid_minecraft_certificates"></ToggleSwitch>
        </setting-item>
        <setting-item
          title="忽略补丁差异"
          description="将 <code>-Dfml.ignorePatchDiscrepancies=true</code> 添加到 JVM 参数中">
          <ToggleSwitch v-model="config.launch.ignore_patch_discrepancies"></ToggleSwitch>
        </setting-item>
        <setting-item title="LWJGL 设置(TODO)" description="" :clickAble="true">
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
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/config";
const config = useConfigStore();
</script>

<style lang="less"></style>
