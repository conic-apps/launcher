<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="'基础设置'">
      <SettingItem :title="'窗口大小'" :description="'游戏窗口的初始大小'">
        <BaseInput
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="'宽'"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.width"
          :lazy-update-value="true">
        </BaseInput>
        <BaseInput
          width="100px"
          style="display: inline-block"
          :placeholder="'高'"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.height"
          :lazy-update-value="true">
        </BaseInput>
        <span style="font-size: 12px; margin-left: 8px">{{ "全屏" }}: </span>
        <BaseSwitch v-model="config.launch.fullscreen"></BaseSwitch>
      </SettingItem>
      <SettingItem :title="'启动游戏后退出启动器'">
        <BaseSwitch v-model="config.launch.quit_app_after_launch"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="'内存'">
      <SettingItem :title="'自动分配内存'">
        <BaseSwitch v-model="config.launch.auto_memory"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="'手动分配内存'"
        :description="'手动指定 Java 堆的最大大小，关闭自动分配后生效'"
        :disabled="config.launch.auto_memory">
        <BaseInput
          width="100px"
          style="display: inline-block; margin-right: 8px"
          :number-only="true"
          :disabled="config.launch.auto_memory"
          v-model.number="config.launch.max_memory"
          :lazy-update-model="true">
        </BaseInput>
        <span style="font-size: 12px">MB</span>
      </SettingItem>
    </SettingGroup>
    <SettingCollapse
      :title="'高级启动选项'"
      :resetable="advancedLaunchOptionsChanged"
      @reset="resetAdvanceOptions">
      <SettingItem :title="'Java 垃圾回收器'">
        <BaseSelect
          :display-name="['G1', 'Z', 'Parallel', 'Serial']"
          :options="['G1', 'Z', 'Parallel', 'Serial']"
          v-model="config.launch.gc"></BaseSelect>
      </SettingItem>
      <SettingItem :title="'添加 JVM 参数'" :description="'将会放在默认 JVM 参数后面'">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_jvm_args"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem :title="'添加游戏参数'" :description="'添加游戏需要的其他参数'">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_mc_args"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem :title="'添加类路径'" :description="'Windows 下用分号隔开，其他系统用冒号'">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_class_paths"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem :title="'启动前执行'" :description="'将会添加至启动命令的前一行'">
        <BaseInput
          width="260px"
          v-model="config.launch.execute_before_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem :title="'包装命令'" :description="'将会添加至启动命令的开头'">
        <BaseInput
          width="260px"
          v-model="config.launch.wrap_command"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem :title="'启动后执行'" :description="'将会添加至启动脚本的最后一行'">
        <BaseInput
          width="260px"
          v-model="config.launch.execute_after_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="'忽略无效的 Minecraft 凭证'"
        :description="'将 <code>-Dfml.ignoreInvalidMinecraftCertificates=true</code> 添加到 JVM 参数中'">
        <BaseSwitch v-model="config.launch.ignore_invalid_minecraft_certificates"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="'忽略补丁差异'"
        :description="'将 <code>-Dfml.ignorePatchDiscrepancies=true</code> 添加到 JVM 参数中'">
        <BaseSwitch v-model="config.launch.ignore_patch_discrepancies"></BaseSwitch>
      </SettingItem>
      <SettingItem
        title="跳过游戏文件检查"
        description="启动游戏前启动器将不会尝试检查或补全游戏文件">
        <BaseSwitch v-model="config.launch.skip_check_files"></BaseSwitch>
      </SettingItem>
    </SettingCollapse>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import SettingCollapse from "@/components/SettingCollapse.vue";
import BaseInput from "@/components/BaseInput.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
import { computed } from "vue";
import { getDefaultConfig } from "@conic/config";
import BaseSelect from "@/components/BaseSelect.vue";

const config = useConfigStore();

const advancedLaunchOptionsChanged = computed(() => {
  const launchOptions = config.launch;
  const isDefault =
    launchOptions.gc === "G1" &&
    !launchOptions.extra_jvm_args &&
    !launchOptions.extra_mc_args &&
    !launchOptions.extra_class_paths &&
    !launchOptions.execute_before_launch &&
    !launchOptions.wrap_command &&
    !launchOptions.execute_after_launch &&
    !launchOptions.ignore_invalid_minecraft_certificates &&
    !launchOptions.ignore_patch_discrepancies;
  return !isDefault;
});

async function resetAdvanceOptions() {
  const defaultConfig = await getDefaultConfig();
  config.launch.gc = defaultConfig.launch.gc;
  config.launch.extra_jvm_args = defaultConfig.launch.extra_jvm_args;
  config.launch.extra_mc_args = defaultConfig.launch.extra_mc_args;
  config.launch.extra_class_paths = defaultConfig.launch.extra_class_paths;
  config.launch.execute_before_launch = defaultConfig.launch.execute_before_launch;
  config.launch.wrap_command = defaultConfig.launch.wrap_command;
  config.launch.execute_after_launch = defaultConfig.launch.execute_after_launch;
  config.launch.ignore_invalid_minecraft_certificates =
    defaultConfig.launch.ignore_invalid_minecraft_certificates;
  config.launch.ignore_patch_discrepancies = defaultConfig.launch.ignore_patch_discrepancies;
}
</script>

<style lang="less"></style>
