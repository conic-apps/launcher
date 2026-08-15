<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="settings">
    <ScrollView>
      <div class="title">
        <AppIcon name="settings"></AppIcon>
        <p>实例设置</p>
      </div>
      <setting-group title="实例设置">
        <setting-item
          title="Instance Name"
          description="The name of this game instance."
          :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'"
          icon="tag">
          <BaseInput
            v-if="instanceName == 'Latest Release'"
            width="300px"
            :value="'最新版本'"
            :lazy-update-model="true">
          </BaseInput>
          <BaseInput
            v-else-if="instanceName == 'Latest Snapshot'"
            width="300px"
            :value="'最新快照'"
            :lazy-update-model="true">
          </BaseInput>
          <BaseInput
            v-else
            width="300px"
            v-model="instanceStore.currentInstance.config.name"
            :non-empty="true"
            :lazy-update-model="true"
            @updated="$emit('update-instance-list')">
          </BaseInput>
        </setting-item>
        <setting-item title="Icon" description="The icon of this game instance." icon="icons">
          <img width="32px" height="32px" src="@/assets/images/Grass_Block.webp" alt="" />
          <AppIcon name="chevron-forward" style="margin-right: 4px; margin-left: 4px"></AppIcon>
        </setting-item>
        <setting-item
          v-if="instanceName === 'Latest Release'"
          :title="'隐藏\u0022最新版本\u0022'"
          :description="'不在游戏列表中显示\u0022最新版本\u0022'"
          icon="eye-off">
          <BaseButton @click="config.accessibility.hide_latest_release = true">Hide it</BaseButton>
        </setting-item>
        <setting-item
          v-if="instanceName === 'Latest Snapshot'"
          :title="'隐藏\u0022最新快照\u0022'"
          :description="'不在游戏列表中显示\u0022最新快照\u0022'"
          icon="eye-off">
          <BaseButton @click="config.accessibility.hide_latest_snapshot = true">Hide it</BaseButton>
        </setting-item>
        <setting-item
          title="Enable Instance-specific Settings"
          description="Description"
          icon="settings">
          <BaseSwitch
            v-model="
              instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings
            "></BaseSwitch>
        </setting-item>
      </setting-group>
      <setting-group :title="'启动选项'" :disabled="!enableInstanceSpecificSettings">
        <setting-item
          :title="'启动后自动进入存档(TODO)'"
          :description="'游戏版本不低于 1.21 时有效'"
          icon="enter">
          <BaseInput width="300px" :placeholder="'存档文件夹的名称'" :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <SettingItem :title="'窗口大小'" :description="'游戏窗口的初始大小'" icon="resize">
          <BaseInput
            width="100px"
            style="display: inline-block; margin-right: 16px"
            :placeholder="'宽'"
            :number-only="true"
            :disabled="config.launch.fullscreen"
            v-model.number="instanceStore.currentInstance.config.launch_config.width"
            :lazy-update-value="true">
          </BaseInput>
          <BaseInput
            width="100px"
            style="display: inline-block"
            :placeholder="'高'"
            :number-only="true"
            :disabled="config.launch.fullscreen"
            v-model.number="instanceStore.currentInstance.config.launch_config.height"
            :lazy-update-value="true">
          </BaseInput>
          <span style="font-size: 12px; margin-left: 8px">{{ "全屏" }}: </span>
          <BaseSwitch
            v-model="instanceStore.currentInstance.config.launch_config.fullscreen"></BaseSwitch>
        </SettingItem>
        <setting-item :title="'启动游戏后隐藏启动器(TODO)'" icon="eye-off">
          <BaseSwitch></BaseSwitch>
        </setting-item>
        <setting-item
          :title="'自动分配内存'"
          :description="'根据系统当前可用内存自动计算游戏内存大小，Mod 越多分配越多'"
          icon="resize">
          <BaseSwitch
            v-model="instanceStore.currentInstance.config.launch_config.auto_memory"></BaseSwitch>
        </setting-item>
        <setting-item
          :title="'手动分配内存'"
          :description="'手动指定 Java 堆的最大大小，关闭自动分配后生效'"
          icon="resize"
          :disabled="instanceStore.currentInstance.config.launch_config.auto_memory">
          <BaseInput
            width="100px"
            style="display: inline-block; margin-right: 8px"
            :number-only="true"
            :disabled="instanceStore.currentInstance.config.launch_config.auto_memory"
            v-model.number="instanceStore.currentInstance.config.launch_config.max_memory"
            :lazy-update-model="true">
          </BaseInput>
          <span style="font-size: 12px">MB</span>
        </setting-item>
      </setting-group>
      <setting-group :title="'高级启动选项'" :disabled="!enableInstanceSpecificSettings">
        <setting-item :title="'Java 垃圾回收器'">
          <BaseDropdownSelect
            :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
            :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
            v-model="instanceStore.currentInstance.config.launch_config.gc"
            :default="0"></BaseDropdownSelect>
        </setting-item>
        <setting-item :title="'添加 JVM 参数'" :description="'将会放在默认 JVM 参数后面'">
          <BaseInput
            width="300px"
            v-model="instanceStore.currentInstance.config.launch_config.extra_jvm_args"
            :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <setting-item :title="'添加游戏参数'" :description="'添加游戏需要的其他参数'">
          <BaseInput
            width="300px"
            v-model="instanceStore.currentInstance.config.launch_config.extra_mc_args"
            :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <setting-item :title="'添加类路径'" :description="'Windows 下用分号隔开，其他系统用冒号'">
          <BaseInput
            width="300px"
            v-model="instanceStore.currentInstance.config.launch_config.extra_class_paths"
            :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <setting-item :title="'启动前执行'" :description="'将会添加至启动命令的前一行'">
          <BaseInput
            width="300px"
            v-model="instanceStore.currentInstance.config.launch_config.execute_before_launch"
            :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <setting-item :title="'包装命令'" :description="'将会添加至启动命令的开头'">
          <BaseInput
            width="300px"
            v-model="instanceStore.currentInstance.config.launch_config.wrap_command"
            :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <setting-item :title="'启动后执行'" :description="'将会添加至启动脚本的最后一行'">
          <BaseInput
            width="300px"
            v-model="instanceStore.currentInstance.config.launch_config.execute_after_launch"
            :lazy-update-model="true">
          </BaseInput>
        </setting-item>
        <setting-item
          :title="'忽略无效的 Minecraft 凭证'"
          :description="'将 <code>-Dfml.ignoreInvalidMinecraftCertificates=true</code> 添加到 JVM 参数中'">
          <BaseSwitch
            v-model="
              instanceStore.currentInstance.config.launch_config
                .ignore_invalid_minecraft_certificates
            ">
          </BaseSwitch>
        </setting-item>
        <setting-item
          :title="'忽略补丁差异'"
          :description="'将 <code>-Dfml.ignorePatchDiscrepancies=true</code> 添加到 JVM 参数中'">
          <BaseSwitch
            v-model="instanceStore.currentInstance.config.launch_config.ignore_patch_discrepancies">
          </BaseSwitch>
        </setting-item>
        <setting-item :title="'LWJGL 设置(TODO)'" description="" :navigable="true"> </setting-item>
      </setting-group>
      <setting-group title="Danger Zone" :danger="true">
        <setting-item
          title="Delete This Instance"
          description="Once you delete a instance, there is no going back. Please be certain."
          icon="trash"
          :navigable="true"
          @click="openDeleteInstanceDialog"
          :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'">
        </setting-item>
        <setting-item
          title="Reset This Instance"
          description="Clear all data in this instance, including worlds, packages, and modules"
          icon="refresh"
          :navigable="true">
        </setting-item>
      </setting-group>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import BaseInput from "@/components/BaseInput.vue";
import { computed, watchEffect } from "vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import BaseDropdownSelect from "@/components/BaseDropdownSelect.vue";
import { useInstanceStore } from "@/store/instance";
import BaseButton from "@/components/BaseButton.vue";
import { updateInstance } from "@conic/instance";
import { useDialogStore } from "@/store/dialog";
import { useInstanceSettings } from "./useGameView";
import ScrollView from "@/components/ScrollView.vue";

const instanceStore = useInstanceStore();
const dialogStore = useDialogStore();

const instanceName = computed(() => {
  return instanceStore.currentInstance.config.name;
});

const config = useConfigStore();

const enableInstanceSpecificSettings = computed(() => {
  return instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings;
});

let oldEnabledSpecificSettings =
  instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings;

watchEffect(() => {
  const currentInstanceConfig = instanceStore.currentInstance.config;
  document.body.classList.add("saving-instance-settings");
  if (
    currentInstanceConfig.launch_config.enable_instance_specific_settings &&
    !oldEnabledSpecificSettings
  ) {
    instanceStore.currentInstance.config.launch_config = {
      enable_instance_specific_settings: true,
      auto_memory: config.launch.auto_memory,
      max_memory: config.launch.max_memory,
      server:
        config.launch.server && config.launch.server.ip
          ? {
              ip: config.launch.server?.ip,
              port: config.launch.server?.port,
            }
          : undefined,
      width: config.launch.width,
      height: config.launch.height,
      fullscreen: config.launch.fullscreen,
      extra_jvm_args: config.launch.extra_jvm_args,
      extra_mc_args: config.launch.extra_mc_args,
      is_demo: config.launch.is_demo,
      ignore_invalid_minecraft_certificates: config.launch.ignore_invalid_minecraft_certificates,
      ignore_patch_discrepancies: config.launch.ignore_patch_discrepancies,
      extra_class_paths: config.launch.extra_class_paths,
      gc: config.launch.gc,
      launcher_name: config.launch.launcher_name,
      wrap_command: config.launch.wrap_command,
      execute_before_launch: config.launch.execute_before_launch,
      execute_after_launch: config.launch.execute_after_launch,
    };
    document.body.classList.remove("saving-instance-settings");
    oldEnabledSpecificSettings =
      currentInstanceConfig.launch_config.enable_instance_specific_settings;
  }
  if (
    !currentInstanceConfig.launch_config.enable_instance_specific_settings &&
    oldEnabledSpecificSettings
  ) {
    instanceStore.currentInstance.config.launch_config = {
      enable_instance_specific_settings: false,
    };
    document.body.classList.remove("saving-instance-settings");
    oldEnabledSpecificSettings =
      currentInstanceConfig.launch_config.enable_instance_specific_settings;
  }
  oldEnabledSpecificSettings =
    currentInstanceConfig.launch_config.enable_instance_specific_settings;
  updateInstance(currentInstanceConfig, instanceStore.currentInstance.id).then(() => {
    document.body.classList.remove("saving-instance-settings");
  });
});

function openDeleteInstanceDialog() {
  useInstanceSettings().value = false;
  dialogStore.confirmDeleteInstance.instanceToDelete = instanceStore.currentInstance;
  dialogStore.confirmDeleteInstance.visible = true;
}
</script>

<style lang="less" scoped>
.settings {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;

  .title {
    width: 100%;
    background: var(--ctp-mantle);
    height: 52px;
    padding: 0 32px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 8px;
  }
}
</style>
