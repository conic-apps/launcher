<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="settings">
    <setting-group>
      <setting-item
        title="Instance Name"
        description="The name of this game instance."
        :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'"
        icon="tag">
        <TextInputBox
          v-if="instanceName == 'Latest Release'"
          width="300px"
          :value="$t('game.latestRelease')"
          :lazy-update-model="true">
        </TextInputBox>
        <TextInputBox
          v-else-if="instanceName == 'Latest Snapshot'"
          width="300px"
          :value="$t('game.latestSnapshot')"
          :lazy-update-model="true">
        </TextInputBox>
        <TextInputBox
          v-else
          width="300px"
          v-model="instanceStore.currentInstance.config.name"
          :non-empty="true"
          :lazy-update-model="true"
          @updated="$emit('update-instance-list')">
        </TextInputBox>
      </setting-item>
      <setting-item title="Icon" description="The name of this game instance." icon="icons">
        <img width="32px" height="32px" src="@/assets/images/Grass_Block.webp" alt="" />
        <i class="chevron-right" style="margin-right: 10px; margin-left: 8px"></i>
      </setting-item>
      <setting-item
        v-if="instanceName === 'Latest Release'"
        :title="$t('settings.accessibility.hideLatestRelease')"
        :description="$t('settings.accessibility.hideLatestReleaseDesc')"
        icon="eye-off">
        <button-vue @click="config.accessibility.hide_latest_release = true">Hide it</button-vue>
      </setting-item>
      <setting-item
        v-if="instanceName === 'Latest Snapshot'"
        :title="$t('settings.accessibility.hideLatestSnapshot')"
        :description="$t('settings.accessibility.hideLatestSnapshotDesc')"
        icon="eye-off">
        <button-vue @click="config.accessibility.hide_latest_snapshot = true">Hide it</button-vue>
      </setting-item>
      <setting-item
        title="Enable Instance-specific Settings"
        description="Description"
        icon="settings">
        <toggle-switch
          v-model="
            instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings
          "></toggle-switch>
      </setting-item>
    </setting-group>
    <setting-group
      :title="$t('settings.game.launchOptions')"
      :disabled="!enableInstanceSpecificSettings">
      <setting-item
        :title="$t('settings.game.launcherName')"
        :description="$t('settings.game.launcherNameDesc')"
        icon="tag">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.launcher_name"
          :placeholder="config.launch.launcher_name"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <!-- TODO:<setting-item title="服务器地址" description="启动后自动加入服务器" icon="server"> -->
      <!--   <TextInputBox -->
      <!--     width="240px" -->
      <!--     v-model="config.launch.server!.ip" -->
      <!--     style="display: inline-block; margin-right: 16px" -->
      <!--     placeholder="IP 或域名" :lazy-update-model="true"></TextInputBox> -->
      <!--   <TextInputBox -->
      <!--     width="100px" -->
      <!--     v-model="config.launch.server!.port" -->
      <!--     placeholder="端口" -->
      <!--     style="display: inline-block" :lazy-update-model="true"></TextInputBox> -->
      <!-- </setting-item> -->
      <setting-item
        :title="$t('settings.game.enterWorldAfterLaunch')"
        :description="$t('settings.game.enterWorldAfterLaunchDesc')"
        icon="enter">
        <TextInputBox
          width="300px"
          :placeholder="$t('settings.game.enterWorldAfterLaunchPlaceholder')"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.game.fullscreen')"
        :description="$t('settings.game.fullscreenDesc')"
        icon="expand">
        <ToggleSwitch
          v-model="instanceStore.currentInstance.config.launch_config.fullscreen"></ToggleSwitch>
      </setting-item>
      <setting-item
        :disabled="instanceStore.currentInstance.config.launch_config.fullscreen"
        :title="$t('settings.game.windowSize')"
        :description="$t('settings.game.windowSizeDesc')"
        icon="resize">
        <TextInputBox
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="$t('settings.game.windowSizeWidth')"
          :number-only="true"
          :disabled="instanceStore.currentInstance.config.launch_config.fullscreen"
          v-model.number="instanceStore.currentInstance.config.launch_config.width"
          :lazy-update-model="true">
        </TextInputBox>
        <TextInputBox
          width="100px"
          style="display: inline-block"
          :placeholder="$t('settings.game.windowSizeHeight')"
          :number-only="true"
          :disabled="instanceStore.currentInstance.config.launch_config.fullscreen"
          v-model.number="instanceStore.currentInstance.config.launch_config.height"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item :title="$t('settings.game.hideLauncherAfterLaunch')" icon="eye-off">
        <toggle-switch></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.demo')"
        :description="$t('settings.game.demoDesc')"
        icon="lock">
        <toggle-switch
          v-model="instanceStore.currentInstance.config.launch_config.is_demo"></toggle-switch>
      </setting-item>
    </setting-group>
    <setting-group
      :title="$t('settings.advance.launchArgs')"
      :disabled="!enableInstanceSpecificSettings">
      <setting-item :title="$t('settings.advance.gc')">
        <select-vue
          :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
          :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
          v-model="instanceStore.currentInstance.config.launch_config.gc"
          :default="0"></select-vue>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraJVMArgs')"
        :description="$t('settings.advance.extraJVMArgsDesc')">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.extra_jvm_args"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraMinecraftArgs')"
        :description="$t('settings.advance.extraMinecraftArgsDesc')">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.extra_mc_args"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraClassPaths')"
        :description="$t('settings.advance.extraClassPathsDesc')">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.extra_class_paths"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.executeBeforeLaunch')"
        :description="$t('settings.advance.executeBeforeLaunchDesc')">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.execute_before_launch"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.wrapCommand')"
        :description="$t('settings.advance.wrapCommandDesc')">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.wrap_command"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.executeAfterLaunch')"
        :description="$t('settings.advance.executeAfterLaunchDesc')">
        <TextInputBox
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.execute_after_launch"
          :lazy-update-model="true">
        </TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.ignoreInvalidMinecraftCertificates')"
        :description="$t('settings.advance.ignoreInvalidMinecraftCertificatesDesc')">
        <ToggleSwitch
          v-model="
            instanceStore.currentInstance.config.launch_config.ignore_invalid_minecraft_certificates
          ">
        </ToggleSwitch>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.ignorePatchDiscrepancies')"
        :description="$t('settings.advance.ignorePatchDiscrepanciesDesc')">
        <ToggleSwitch
          v-model="instanceStore.currentInstance.config.launch_config.ignore_patch_discrepancies">
        </ToggleSwitch>
      </setting-item>
      <setting-item :title="$t('settings.advance.lwjglSettings')" description="" :clickAble="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
      <setting-item
        title="Open Log Viewer"
        description="The Description of Open Log Viewer "
        icon="document-text"
        :clickAble="true"
        @click="logViewerOpen = true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
    <setting-group title="Danger Zone" :danger="true">
      <setting-item
        title="Delete This Instance"
        description="Once you delete a instance, there is no going back. Please be certain."
        icon="trash"
        :clickAble="true"
        @click="confirmDeleteInstanceVisible = true"
        :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
      <setting-item
        title="Reset This Instance"
        description="Clear all data in this instance, including worlds, packages, and modules"
        icon="refresh"
        :clickAble="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
    <confirm-delete-instance
      :visible="confirmDeleteInstanceVisible"
      @close="confirmDeleteInstanceVisible = false"
      @deleted="
        confirmDeleteInstanceVisible = false;
        $emit('update-instance-list');
      "></confirm-delete-instance>
    <LogViewer :visible="logViewerOpen" @close="logViewerOpen = false"> </LogViewer>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import TextInputBox from "@/components/TextInputBox.vue";
import { computed, ref, watchEffect } from "vue";
import ToggleSwitch from "@/components/ToggleSwitch.vue";
import ConfirmDeleteInstance from "../dialogs/ConfirmDeleteInstance.vue";
import SelectVue from "@/components/Select.vue";
import LogViewer from "../dialogs/LogViewer.vue";
import { useInstanceStore } from "@/store/instance";
import ButtonVue from "@/components/Button.vue";
import { updateInstance } from "@conic/instance";

defineEmits(["update-instance-list"]);

const instanceStore = useInstanceStore();

const instanceName = computed(() => {
  return instanceStore.currentInstance.config.name;
});

const config = useConfigStore();

const enableInstanceSpecificSettings = computed(() => {
  return instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings;
});

const confirmDeleteInstanceVisible = ref(false);

const logViewerOpen = ref(false);

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
      min_memory: config.launch.min_memory,
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
</script>

<style lang="less" scoped>
.settings {
  display: flex;
  flex-direction: column;
}
</style>
