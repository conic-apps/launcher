<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.accountAdd.visible" animate-height>
    <AccountAdd @switch-component-manage="dialogStore.accountAdd.visible = false"></AccountAdd>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/BaseDialog.vue";
import AccountAdd from "../views/accounts/AccountAdd.vue";
import { useDialogStore } from "@/store/dialog";

const dialogStore = useDialogStore();
</script>
