<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="accounts-view">
    <AccountManager></AccountManager>
  </div>
</template>

<script setup lang="ts">
import AccountManager from "./accounts/AccountManager.vue";
</script>

<style lang="less" scoped>
.accounts-view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}
</style>
