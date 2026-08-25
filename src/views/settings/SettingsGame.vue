<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div>
    <SettingGroup :title="t('settings.game.basicSettings')">
      <SettingItem
        :title="t('settings.game.windowSize')"
        :description="t('settings.game.windowSizeDesc')">
        <BaseInput
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="t('settings.game.width')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.width"
          :lazy-update-model="true">
        </BaseInput>
        <BaseInput
          width="100px"
          style="display: inline-block"
          :placeholder="t('settings.game.height')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.height"
          :lazy-update-model="true">
        </BaseInput>
        <span style="font-size: 12px; margin-left: 8px">{{ t("settings.game.fullscreen") }}: </span>
        <BaseSwitch v-model="config.launch.fullscreen"></BaseSwitch>
      </SettingItem>
      <SettingItem :title="t('settings.game.quitAfterLaunch')">
        <BaseSwitch v-model="config.launch.quit_app_after_launch"></BaseSwitch>
      </SettingItem>
    </SettingGroup>
    <SettingGroup :title="t('settings.game.memory')">
      <SettingItem :title="t('settings.game.autoMemory')">
        <BaseSwitch v-model="config.launch.auto_memory"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.manualMemory')"
        :description="t('settings.game.manualMemoryDesc')"
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
      :title="t('settings.game.advancedOptions')"
      :resetable="advancedLaunchOptionsChanged"
      @reset="resetAdvanceOptions">
      <SettingItem :title="t('settings.game.gc')">
        <BaseSelect
          :display-name="['G1', 'Z', 'Parallel', 'Serial']"
          :options="['G1', 'Z', 'Parallel', 'Serial']"
          v-model="config.launch.gc"></BaseSelect>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.jvmArgs')"
        :description="t('settings.game.jvmArgsDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_jvm_args"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.gameArgs')"
        :description="t('settings.game.gameArgsDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_mc_args"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.classPath')"
        :description="t('settings.game.classPathDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.extra_class_paths"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.beforeLaunch')"
        :description="t('settings.game.beforeLaunchDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.execute_before_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.wrapCommand')"
        :description="t('settings.game.wrapCommandDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.wrap_command"
          :lazy-update-model="true"></BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.afterLaunch')"
        :description="t('settings.game.afterLaunchDesc')">
        <BaseInput
          width="260px"
          v-model="config.launch.execute_after_launch"
          :lazy-update-model="true">
        </BaseInput>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.ignoreInvalidCerts')"
        :description="t('settings.game.ignoreInvalidCertsDesc')">
        <BaseSwitch v-model="config.launch.ignore_invalid_minecraft_certificates"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.ignorePatchDiff')"
        :description="t('settings.game.ignorePatchDiffDesc')">
        <BaseSwitch v-model="config.launch.ignore_patch_discrepancies"></BaseSwitch>
      </SettingItem>
      <SettingItem
        :title="t('settings.game.skipFileCheck')"
        :description="t('settings.game.skipFileCheckDesc')">
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
import { useI18n } from "vue-i18n";

const { t } = useI18n();
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
