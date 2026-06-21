<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="settings">
    <SettingGroup>
      <SettingItem
        title="Instance Name"
        description="The name of this game instance."
        :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'"
        icon="tag">
        <BaseInput
          v-if="instanceName == 'Latest Release'"
          width="300px"
          :value="$t('game.latestRelease')"
          :lazy-update-model="true">
        </BaseInput>
        <BaseInput
          v-else-if="instanceName == 'Latest Snapshot'"
          width="300px"
          :value="$t('game.latestSnapshot')"
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
      </SettingItem>
      <SettingItem title="Icon" description="The icon of this game instance." icon="icons">
        <img width="32px" height="32px" src="@/assets/images/Grass_Block.webp" alt="" />
        <AppIcon name="chevron-forward" style="margin-right: 4px; margin-left: 4px"></AppIcon>
      </SettingItem>
      <SettingItem
        v-if="instanceName === 'Latest Release'"
        :title="$t('settings.accessibility.hideLatestRelease')"
        :description="$t('settings.accessibility.hideLatestReleaseDesc')"
        icon="eye-off">
        <BaseButton @click="config.accessibility.hide_latest_release = true">Hide it</BaseButton>
      </SettingItem>
      <SettingItem
        v-if="instanceName === 'Latest Snapshot'"
        :title="$t('settings.accessibility.hideLatestSnapshot')"
        :description="$t('settings.accessibility.hideLatestSnapshotDesc')"
        icon="eye-off">
        <BaseButton @click="config.accessibility.hide_latest_snapshot = true">Hide it</BaseButton>
      </SettingItem>
      <SettingItem
        title="Enable Instance-specific Settings"
        description="Description"
        icon="settings">
        <BaseSwitch
          v-model="
            instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings
          "></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingGroup
      :title="$t('settings.game.launchOptions')"
      :disabled="!enableInstanceSpecificSettings">
      <SettingItem
        :title="$t('settings.game.launcherName')"
        :description="$t('settings.game.launcherNameDesc')"
        icon="tag">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.launcher_name"
          :placeholder="config.launch.launcher_name"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <!-- TODO:<SettingItem title="服务器地址" description="启动后自动加入服务器" icon="server"> -->
      <!--   <BaseInput -->
      <!--     width="240px" -->
      <!--     v-model="config.launch.server!.ip" -->
      <!--     style="display: inline-block; margin-right: 16px" -->
      <!--     placeholder="IP 或域名" :lazy-update-model="true"></BaseInput> -->
      <!--   <BaseInput -->
      <!--     width="100px" -->
      <!--     v-model="config.launch.server!.port" -->
      <!--     placeholder="端口" -->
      <!--     style="display: inline-block" :lazy-update-model="true"></BaseInput> -->
      <!-- </SettingItem> -->
      <SettingItem
        :title="$t('settings.game.enterWorldAfterLaunch')"
        :description="$t('settings.game.enterWorldAfterLaunchDesc')"
        icon="enter">
        <BaseInput
          width="300px"
          :placeholder="$t('settings.game.enterWorldAfterLaunchPlaceholder')"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.fullscreen')"
        :description="$t('settings.game.fullscreenDesc')"
        icon="expand">
        <BaseSwitch
          v-model="instanceStore.currentInstance.config.launch_config.fullscreen"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :disabled="instanceStore.currentInstance.config.launch_config.fullscreen"
        :title="$t('settings.game.windowSize')"
        :description="$t('settings.game.windowSizeDesc')"
        icon="resize">
        <BaseInput
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="$t('settings.game.windowSizeWidth')"
          :number-only="true"
          :disabled="instanceStore.currentInstance.config.launch_config.fullscreen"
          v-model.number="instanceStore.currentInstance.config.launch_config.width"
          :lazy-update-model="true">
          <!-- BUG: Frontend crash when user clear this input box -->
        </BaseInput>
        <BaseInput
          width="100px"
          style="display: inline-block"
          :placeholder="$t('settings.game.windowSizeHeight')"
          :number-only="true"
          :disabled="instanceStore.currentInstance.config.launch_config.fullscreen"
          v-model.number="instanceStore.currentInstance.config.launch_config.height"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem :title="$t('settings.game.hideLauncherAfterLaunch')" icon="eye-off">
        <BaseSwitch></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.demo')"
        :description="$t('settings.game.demoDesc')"
        icon="lock">
        <BaseSwitch
          v-model="instanceStore.currentInstance.config.launch_config.is_demo"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingGroup
      :title="$t('settings.advance.launchArgs')"
      :disabled="!enableInstanceSpecificSettings">
      <SettingItem :title="$t('settings.advance.gc')">
        <BaseSelect
          :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
          :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
          v-model="instanceStore.currentInstance.config.launch_config.gc"
          :default="0"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.extraJVMArgs')"
        :description="$t('settings.advance.extraJVMArgsDesc')">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.extra_jvm_args"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.extraMinecraftArgs')"
        :description="$t('settings.advance.extraMinecraftArgsDesc')">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.extra_mc_args"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.extraClassPaths')"
        :description="$t('settings.advance.extraClassPathsDesc')">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.extra_class_paths"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.executeBeforeLaunch')"
        :description="$t('settings.advance.executeBeforeLaunchDesc')">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.execute_before_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.wrapCommand')"
        :description="$t('settings.advance.wrapCommandDesc')">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.wrap_command"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.executeAfterLaunch')"
        :description="$t('settings.advance.executeAfterLaunchDesc')">
        <BaseInput
          width="300px"
          v-model="instanceStore.currentInstance.config.launch_config.execute_after_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.ignoreInvalidMinecraftCertificates')"
        :description="$t('settings.advance.ignoreInvalidMinecraftCertificatesDesc')">
        <BaseSwitch
          v-model="
            instanceStore.currentInstance.config.launch_config.ignore_invalid_minecraft_certificates
          ">
        </BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.ignorePatchDiscrepancies')"
        :description="$t('settings.advance.ignorePatchDiscrepanciesDesc')">
        <BaseSwitch
          v-model="instanceStore.currentInstance.config.launch_config.ignore_patch_discrepancies">
        </BaseSwitch>
      </SettingItem>
      <SettingItem :title="$t('settings.advance.lwjglSettings')" description="" :navigable="true">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </SettingItem>
      <SettingItem
        title="Open Log Viewer"
        description="The Description of Open Log Viewer "
        icon="document-text"
        :navigable="true"
        @click="dialogStore.logViewer.visible = true">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </SettingItem>
    </SettingGroup>
    <SettingGroup title="Danger Zone" :danger="true">
      <SettingItem
        title="Delete This Instance"
        description="Once you delete a instance, there is no going back. Please be certain."
        icon="trash"
        :navigable="true"
        @click="dialogStore.confirmDeleteInstance.visible = true"
        :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </SettingItem>
      <SettingItem
        title="Reset This Instance"
        description="Clear all data in this instance, including worlds, packages, and modules"
        icon="refresh"
        :navigable="true">
        <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseInput from "@/components/base/BaseInput.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import BaseSelect from "@/components/base/BaseSelect.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import { useConfigStore } from "@/store/config";
import { useInstanceStore } from "@/store/instance";
import { updateInstance } from "@conic/instance";
import { useDialogStore } from "@/store/dialog";
import { computed, watchEffect } from "vue";

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
