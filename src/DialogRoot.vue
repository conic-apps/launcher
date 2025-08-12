<template>
  <UpdateReminder :visible="dialogStore.updateReminder"></UpdateReminder>
  <AccountManager :visible="dialogStore.accountManager"></AccountManager>
  <CreateInstance :visible="dialogStore.createInstance">></CreateInstance>
  <LogViewer :visible="dialogStore.logViewer">></LogViewer>
  <ConfirmDeleteInstance :visible="dialogStore.confirmDeleteInstance">></ConfirmDeleteInstance>
</template>

<script setup lang="ts">
import UpdateReminder from "./dialogs/UpdateReminder.vue";
import AccountManager from "./dialogs/AccountManager.vue";
import CreateInstance from "./dialogs/CreateInstance.vue";
import LogViewer from "./dialogs/LogViewer.vue";
import ConfirmDeleteInstance from "./dialogs/ConfirmDeleteInstance.vue";
import { useDialogStore } from "./store/dialog";

const dialogStore = useDialogStore();
</script>
