<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <UpdateReminder></UpdateReminder>
  <AccountManager></AccountManager>
  <CreateInstance></CreateInstance>
  <LogViewer></LogViewer>
  <ConfirmDeleteInstance></ConfirmDeleteInstance>
</template>

<script setup lang="ts">
import UpdateReminder from "./dialogs/UpdateReminder.vue";
import AccountManager from "./dialogs/AccountManager.vue";
import CreateInstance from "./dialogs/CreateInstance.vue";
import LogViewer from "./dialogs/LogViewer.vue";
import ConfirmDeleteInstance from "./dialogs/ConfirmDeleteInstance.vue";
</script>
