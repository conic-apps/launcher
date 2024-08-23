<template>
  <keep-alive>
    <div>
      <setting-group :title="$t('settings.accessibility.extraFeatures')">
        <setting-item :title="$t('settings.accessibility.releaseReminder')"
          :description="$t('settings.accessibility.releaseReminderDesc')" icon="circle-exclamation">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item :title="$t('settings.accessibility.snapshotReminder')"
          :description="$t('settings.accessibility.snapshotReminderDesc')" icon="circle-exclamation">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item :title="$t('settings.accessibility.hideLatestRelease')"
          :description="$t('settings.accessibility.hideLatestReleaseDesc')" icon="eye-slash" :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item :title="$t('settings.accessibility.hideLatestSnapshot')"
          :description="$t('settings.accessibility.hideLatestSnapshotDesc')" icon="eye-slash" :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item :title="$t('settings.accessibility.changeGameLanguage')"
          :description="$t('settings.accessibility.changeGameLanguageDesc')" icon="language" :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
      </setting-group>
      <setting-group :title="$t('settings.accessibility.accessibility')">
        <setting-item :title="$t('settings.accessibility.disableAllAnimations')"
          :description="$t('settings.accessibility.disableAllAnimationsDesc')" icon="pause">
          <toggle-switch></toggle-switch>
        </setting-item>
        <setting-item :title="$t('settings.accessibility.highContrastMode')"
          :description="$t('settings.accessibility.highContrastModeDesc')" icon="laptop" :last="true">
          <toggle-switch></toggle-switch>
        </setting-item>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
</script>

<style lang="less" scoped></style>
