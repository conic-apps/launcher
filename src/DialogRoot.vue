<template>
  <UpdateReminder></UpdateReminder>
  <AccountManager></AccountManager>
  <CreateInstance>></CreateInstance>
  <LogViewer>></LogViewer>
  <ConfirmDeleteInstance></ConfirmDeleteInstance>
</template>

<script setup lang="ts">
import UpdateReminder from "./dialogs/UpdateReminder.vue";
import AccountManager from "./dialogs/AccountManager.vue";
import CreateInstance from "./dialogs/CreateInstance.vue";
import LogViewer from "./dialogs/LogViewer.vue";
import ConfirmDeleteInstance from "./dialogs/ConfirmDeleteInstance.vue";
</script>
