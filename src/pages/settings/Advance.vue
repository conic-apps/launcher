<template>
  <div>
    <setting-group :title="$t('settings.advance.launchArgs')">
      <setting-item :title="$t('settings.advance.gc')">
        <select-vue
          :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
          :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
          v-model="config.launch.gc"
          :default="0"></select-vue>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraJVMArgs')"
        :description="$t('settings.advance.extraJVMArgsDesc')">
        <TextInputBox width="360px" v-model="config.launch.extra_jvm_args"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraMinecraftArgs')"
        :description="$t('settings.advance.extraMinecraftArgsDesc')">
        <TextInputBox width="360px" v-model="config.launch.extra_mc_args"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraClassPaths')"
        :description="$t('settings.advance.extraClassPathsDesc')">
        <TextInputBox width="360px" v-model="config.launch.extra_class_paths"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.executeBeforeLaunch')"
        :description="$t('settings.advance.executeBeforeLaunchDesc')">
        <TextInputBox width="360px" v-model="config.launch.execute_before_launch"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.wrapCommand')"
        :description="$t('settings.advance.wrapCommandDesc')">
        <TextInputBox width="360px" v-model="config.launch.wrap_command"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.executeAfterLaunch')"
        :description="$t('settings.advance.executeAfterLaunchDesc')">
        <TextInputBox width="360px" v-model="config.launch.execute_after_launch"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.ignoreInvalidMinecraftCertificates')"
        :description="$t('settings.advance.ignoreInvalidMinecraftCertificatesDesc')">
        <ToggleSwitch v-model="config.launch.ignore_invalid_minecraft_certificates"></ToggleSwitch>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.ignorePatchDiscrepancies')"
        :description="$t('settings.advance.ignorePatchDiscrepanciesDesc')">
        <ToggleSwitch v-model="config.launch.ignore_patch_discrepancies"></ToggleSwitch>
      </setting-item>
      <setting-item :title="$t('settings.advance.lwjglSettings')" description="" :clickAble="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SelectVue from "@/components/controllers/Select.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
const config = useConfigStore();
</script>

<style lang="less"></style>
