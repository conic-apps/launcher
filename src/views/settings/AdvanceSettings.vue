<!-- Conic Launcher -->
<!-- Copyright 2022-2026 OakChaser and contributors. All rights reserved. -->
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
  </div>
</template>

<script setup lang="ts">
import SettingGroup from "@/components/SettingGroup.vue";
import SettingItem from "@/components/SettingItem.vue";
import { getDataLocation } from "@conic/folder";
import { invoke } from "@tauri-apps/api/core";

async function openLogFolder() {
  const dataLocation = await getDataLocation();
  invoke("open_path", { path: dataLocation.logs });
}
</script>

<style lang="less"></style>
