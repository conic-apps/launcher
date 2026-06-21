<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup>
      <SettingItem
        :title="$t('settings.advance.viewLauncherLogs')"
        :description="$t('settings.advance.viewLauncherLogsDesc')"
        icon="document-text"
        :navigable="true"
        @click="openLogFolder">
      </SettingItem>
    </SettingGroup>
    <SettingGroup
      :title="$t('settings.advance.launchArgs')"
      :resetable="advancedLaunchOptionsChanged"
      @reset="resetAdvanceOptions">
      <SettingItem :title="$t('settings.advance.gc')">
        <BaseSelect
          :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
          :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
          v-model="config.launch.gc"
          :default="0"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.extraJVMArgs')"
        :description="$t('settings.advance.extraJVMArgsDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_jvm_args"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.extraMinecraftArgs')"
        :description="$t('settings.advance.extraMinecraftArgsDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_mc_args"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.extraClassPaths')"
        :description="$t('settings.advance.extraClassPathsDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_class_paths"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.executeBeforeLaunch')"
        :description="$t('settings.advance.executeBeforeLaunchDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.execute_before_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.wrapCommand')"
        :description="$t('settings.advance.wrapCommandDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.wrap_command"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.executeAfterLaunch')"
        :description="$t('settings.advance.executeAfterLaunchDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.execute_after_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.ignoreInvalidMinecraftCertificates')"
        :description="$t('settings.advance.ignoreInvalidMinecraftCertificatesDesc')">
        <BaseSwitch v-model="config.launch.ignore_invalid_minecraft_certificates"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.ignorePatchDiscrepancies')"
        :description="$t('settings.advance.ignorePatchDiscrepanciesDesc')">
        <BaseSwitch v-model="config.launch.ignore_patch_discrepancies"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.advance.lwjglSettings')"
        description="May cause launch failure. For advanced users only."
        :navigable="true">
      </SettingItem>
    </SettingGroup>
  </div>
</template>

<script setup lang="ts">
import SettingGroup from "@/components/SettingGroup.vue";
import SettingItem from "@/components/SettingItem.vue";
import BaseSelect from "@/components/base/BaseSelect.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import BaseInput from "@/components/base/BaseInput.vue";
import { useConfigStore } from "@/store/config";
import { getDataLocation } from "@conic/folder";
import { invoke } from "@tauri-apps/api/core";
import { computed } from "vue";
const config = useConfigStore();

async function openLogFolder() {
  const dataLocation = await getDataLocation();
  invoke("open_path", { path: dataLocation.logs });
}

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

function resetAdvanceOptions() {}
</script>

<style lang="less"></style>
