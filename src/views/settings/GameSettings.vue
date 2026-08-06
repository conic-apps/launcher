<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="$t('settings.game.launchOptions')">
      <SettingItem
        :title="$t('settings.game.windowSize')"
        :description="$t('settings.game.windowSizeDesc')"
        icon="resize">
        <BaseInput
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="$t('settings.game.windowSizeWidth')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.width"
          :lazy-update-value="true">
        </BaseInput>
        <BaseInput
          width="100px"
          style="display: inline-block"
          :placeholder="$t('settings.game.windowSizeHeight')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.height"
          :lazy-update-value="true">
        </BaseInput>
        <span style="font-size: 12px; margin-left: 8px"
          >{{ $t("settings.game.fullscreen") }}:
        </span>
        <BaseSwitch v-model="config.launch.fullscreen"></BaseSwitch>
      </SettingItem>
      <SettingItem :title="$t('settings.game.hideLauncherAfterLaunch')" icon="eye-off">
        <BaseSwitch></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.autoRefreshAccount')"
        :description="$t('settings.game.autoRefreshAccountDesc')"
        icon="refresh">
        <BaseSwitch v-model="config.launch.skip_refresh_account"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="$t('settings.game.autoCompleteGameFiles')"
        :description="$t('settings.game.autoCompleteGameFilesDesc')"
        icon="build">
        <BaseSwitch v-model="config.launch.skip_check_files"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingGroup
      :title="$t('settings.advance.launchArgs')"
      :resetable="advancedLaunchOptionsChanged"
      @reset="resetAdvanceOptions">
      <SettingItem :title="$t('settings.advance.gc')">
        <BaseDropdownSelect
          :display-name="['G1GC', 'ZGC', 'ParallelGC', 'SerialGC']"
          :options="['G1', 'Z', 'Parallel', 'Serial']"
          v-model="config.launch.gc"
          :default="0"></BaseDropdownSelect>
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
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseInput from "@/components/base/BaseInput.vue";
import BaseSwitch from "@/components/base/BaseSwitch.vue";
import { useConfigStore } from "@/store/config";
import { computed } from "vue";
import { getDefaultConfig } from "@conic/config";
import BaseDropdownSelect from "@/components/base/BaseDropdownSelect.vue";

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
