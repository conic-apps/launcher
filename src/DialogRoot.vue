<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <UpdateReminder></UpdateReminder>
  <CreateInstance></CreateInstance>
  <ConfirmDeleteInstance></ConfirmDeleteInstance>
  <ConfirmDeleteAccount></ConfirmDeleteAccount>
  <ConfirmDeleteSave></ConfirmDeleteSave>
  <UploadSkin></UploadSkin>
  <ConfirmQuitApp></ConfirmQuitApp>
  <NoAccountError></NoAccountError>
  <MultiplayerExtension></MultiplayerExtension>
</template>

<script setup lang="ts">
import UpdateReminder from "./dialogs/UpdateReminder.vue";
import CreateInstance from "./dialogs/CreateInstance.vue";
import ConfirmDeleteInstance from "./dialogs/ConfirmDeleteInstance.vue";
import ConfirmDeleteAccount from "./dialogs/ConfirmDeleteAccount.vue";
import ConfirmDeleteSave from "./dialogs/ConfirmDeleteSave.vue";
import UploadSkin from "./dialogs/UploadSkin.vue";
import ConfirmQuitApp from "./dialogs/ConfirmQuitApp.vue";
import NoAccountError from "./dialogs/NoAccountError.vue";
import MultiplayerExtension from "./dialogs/MultiplayerExtension.vue";
</script>
