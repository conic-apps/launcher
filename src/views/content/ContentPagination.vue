<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="pagination" v-if="totalPages > 1">
    <button class="page-nav" :disabled="currentPage === 1" @click="$emit('page-change', currentPage - 1)">
      <AppIcon name="chevron-back" :size="12"></AppIcon>
    </button>
    <template v-for="(page, index) in pages" :key="index">
      <button
        v-if="page !== '…'"
        class="page-number"
        :class="{ active: page === currentPage }"
        @click="$emit('page-change', page)">
        {{ page }}
      </button>
      <span v-else class="page-ellipsis">…</span>
    </template>
    <button
      class="page-nav"
      :disabled="currentPage === totalPages"
      @click="$emit('page-change', currentPage + 1)">
      <AppIcon name="chevron-forward" :size="12"></AppIcon>
    </button>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";

defineProps<{
  totalPages: number;
  currentPage: number;
  pages: (number | "…")[];
}>();

defineEmits<{
  "page-change": [page: number];
}>();
</script>

<style lang="less" scoped>
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 20px 0 32px;

  button {
    min-width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--ctp-surface1);
    border-radius: 6px;
    background: var(--ctp-surface0);
    color: var(--ctp-text);
    font-size: 12px;
    transition:
      background 120ms ease,
      border-color 120ms ease;

    &:hover:not(:disabled),
    &:hover:not(.active) {
      background: var(--ctp-surface1);
    }

    &:active:not(:disabled),
    &:hover:not(.active) {
      background: var(--ctp-surface2);
    }

    &:disabled {
      opacity: 0.4;
    }

    &.active {
      border-color: var(--ctp-lavender);
      background: var(--ctp-lavender);
      color: var(--ctp-text-inverse);
      pointer-events: none;
    }
  }

  .page-ellipsis {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 30px;
    font-size: 12px;
    color: var(--ctp-subtext0);
  }
}
</style>
