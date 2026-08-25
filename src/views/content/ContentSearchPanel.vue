<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="search-panel">
    <div class="search-bar">
      <input
        class="search-input"
        type="text"
        :placeholder="placeholder"
        :value="modelValue"
        @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        autocapitalize="off"
        autocomplete="off"
        autocorrect="off"
        @keyup.enter="$emit('search')" />
      <button class="search-button" @click="$emit('search')">
        <AppIcon name="search" :size="16"></AppIcon>
      </button>
    </div>
    <div class="filter-bar">
      <div class="filter-row" v-for="filter in filters" :key="filter.key">
        <span class="filter-label">{{ filter.label }}</span>
        <div class="filter-chips" :class="{ paged: filter.key === 'version' }">
          <template v-if="filter.key === 'version'">
            <button
              class="chip-pager"
              :disabled="versionPage <= 0"
              @click="$emit('version-page-prev')">
              <AppIcon name="chevron-back" :size="12"></AppIcon>
            </button>
            <div class="filter-chips-track" :ref="setVersionTrackRef">
              <div class="filter-chips-track-inner" :style="versionTrackStyle">
                <button
                  class="filter-chip"
                  :class="getChipClasses(filter, option)"
                  v-for="(option, index) in filter.options"
                  :key="`${filter.key}-${index}`"
                  @click="$emit('filter-change', filter, option)">
                  {{ filter.display(option) }}
                </button>
              </div>
            </div>
            <button
              class="chip-pager"
              :disabled="versionPage >= versionPageCount - 1"
              @click="$emit('version-page-next')">
              <AppIcon name="chevron-forward" :size="12"></AppIcon>
            </button>
          </template>
          <template v-else>
            <button
              class="filter-chip"
              :class="getChipClasses(filter, option)"
              v-for="(option, index) in filter.options"
              :key="`${filter.key}-${index}`"
              @click="$emit('filter-change', filter, option)">
              {{ filter.display(option) }}
            </button>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";

export type ContentFilterItem = {
  key: string;
  label: string;
  options: unknown[];
  isSelected: (option: unknown) => boolean;
  display: (option: unknown) => string;
  chipClass?: (option: unknown) => Record<string, boolean>;
};

defineProps<{
  modelValue: string;
  placeholder?: string;
  filters: ContentFilterItem[];
  versionPage: number;
  versionPageCount: number;
  versionTrackStyle: Record<string, string>;
  setVersionTrackRef: (el: unknown) => void;
}>();

defineEmits<{
  "update:modelValue": [value: string];
  search: [];
  "filter-change": [filter: ContentFilterItem, option: unknown];
  "version-page-prev": [];
  "version-page-next": [];
}>();

function getChipClasses(filter: ContentFilterItem, option: unknown): Record<string, boolean> {
  const base: Record<string, boolean> = { selected: filter.isSelected(option) };
  if (filter.chipClass) {
    return { ...base, ...filter.chipClass(option) };
  }
  return base;
}
</script>

<style lang="less" scoped>
.search-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 16px;
  padding: 24px;
  background: rgba(var(--ctp-crust-rgb), 0.92);
  backdrop-filter: blur(4px);

  .search-bar {
    display: flex;
    gap: 8px;

    .search-input {
      flex: 1;
      height: 36px;
      padding: 0 12px;
      border: 1px solid var(--ctp-surface1);
      border: none;
      border-radius: 8px;
      background: var(--ctp-surface0);
      color: var(--ctp-text);
      font-size: 13px;
      transition:
        background 120ms ease,
        border-color 150ms ease;

      &::placeholder {
        color: var(--ctp-subtext0);
      }

      &:hover {
        background: var(--ctp-surface1);
      }

      &:focus {
        outline: none;
        border-color: var(--ctp-lavender);
      }
    }

    .search-button {
      flex-shrink: 0;
      width: 36px;
      height: 36px;
      display: flex;
      align-items: center;
      justify-content: center;
      border: none;
      border-radius: 8px;
      background: var(--ctp-surface0);
      color: var(--ctp-text);
      transition: background 120ms ease;

      &:hover {
        background: var(--ctp-surface1);
      }

      &:active {
        background: var(--ctp-lavender);
        color: var(--ctp-text-inverse);
      }
    }
  }

  .filter-bar {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: 10px;

    .filter-label {
      flex-shrink: 0;
      min-width: 52px;
      line-height: 26px;
      font-size: 12px;
      color: var(--ctp-subtext0);
    }

    .filter-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;

      &.paged {
        flex: 1;
        min-width: 0;
        flex-wrap: nowrap;
        align-items: center;
      }
    }

    .filter-chips-track {
      flex: 1;
      min-width: 0;
      overflow: hidden;
    }

    .filter-chips-track-inner {
      display: flex;
      flex-wrap: nowrap;
      gap: 6px;
      width: max-content;
      will-change: transform;

      .filter-chip {
        flex-shrink: 0;
      }
    }
  }

  .chip-pager {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--ctp-surface1);
    border-radius: 999px;
    background: var(--ctp-surface0);
    color: var(--ctp-text);
    font-size: 14px;
    transition:
      background 120ms ease,
      color 120ms ease;

    &:hover:not(:disabled) {
      background: var(--ctp-surface1);
    }

    &:disabled {
      opacity: 0.4;
    }
  }

  .filter-chip {
    height: 20px;
    padding: 0 8px;
    border: 1px solid var(--ctp-surface1);
    border: none;
    border-radius: 999px;
    background: var(--ctp-surface0);
    background: none;
    color: var(--ctp-text);
    font-size: 12px;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease;

    &:hover {
      background: var(--ctp-surface1);
      transition: none;
    }

    &:active {
      background: var(--ctp-surface2);
      transition:
        background 120ms ease,
        border-color 120ms ease,
        color 120ms ease;
    }

    &.selected {
      background: var(--ctp-lavender);
      color: var(--ctp-text-inverse);
      transition:
        background 120ms ease,
        border-color 120ms ease,
        color 120ms ease;
    }

    &.selected.fabric {
      background: var(--ctp-yellow);
    }
    &.selected.forge {
      background: var(--ctp-blue);
    }
    &.selected.neoforge {
      background: var(--ctp-peach);
    }
    &.selected.quilt {
      background: var(--ctp-mauve);
    }
    &.selected.minecraft-version {
      background: var(--ctp-green);
    }
  }
}
</style>
