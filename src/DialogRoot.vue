<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <UpdateReminder></UpdateReminder>
  <UpdateApp></UpdateApp>
  <CreateInstance></CreateInstance>
  <ConfirmDeleteInstance></ConfirmDeleteInstance>
  <ConfirmDeleteAccount></ConfirmDeleteAccount>
  <ConfirmDeleteSave></ConfirmDeleteSave>
  <UploadSkin></UploadSkin>
  <ConfirmQuitApp></ConfirmQuitApp>
  <NoAccountError></NoAccountError>
  <NoMicrosoftAccountError></NoMicrosoftAccountError>
  <NoSuitableJavaError></NoSuitableJavaError>
  <MultiplayerExtension></MultiplayerExtension>
  <AccountAdd></AccountAdd>
</template>

<script setup lang="ts">
import UpdateReminder from "./dialogs/UpdateReminder.vue";
import UpdateApp from "./dialogs/UpdateApp.vue";
import CreateInstance from "./dialogs/CreateInstance.vue";
import ConfirmDeleteInstance from "./dialogs/ConfirmDeleteInstance.vue";
import ConfirmDeleteAccount from "./dialogs/ConfirmDeleteAccount.vue";
import ConfirmDeleteSave from "./dialogs/ConfirmDeleteSave.vue";
import UploadSkin from "./dialogs/UploadSkin.vue";
import ConfirmQuitApp from "./dialogs/ConfirmQuitApp.vue";
import NoAccountError from "./dialogs/NoAccountError.vue";
import NoSuitableJavaError from "./dialogs/NoSuitableJavaError.vue";
import MultiplayerExtension from "./dialogs/MultiplayerExtension.vue";
import AccountAdd from "./dialogs/AccountAdd.vue";
import NoMicrosoftAccountError from "./dialogs/NoMicrosoftAccountError.vue";
</script>
