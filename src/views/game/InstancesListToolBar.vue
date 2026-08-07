<template>
  <div class="tool-bar">
    <div class="search">
      <div class="search-input"><input type="text" placeholder="搜索..." /></div>
      <button class="search-button">
        <AppIcon name="search"></AppIcon>
      </button>
    </div>
    <div class="other">
      <div class="sort">
        <InstanceListDropdown label="排序" :selected="sortLabel" :selected-width="102">
          <li
            class="dropdown-option"
            v-for="option in sortOptions"
            :key="option.key"
            :class="{ selected: sortMode === option.key }"
            @click="selectSort(option.key)">
            {{ option.label }}
          </li>
        </InstanceListDropdown>
      </div>
      <div class="group">
        <InstanceListDropdown label="分组" :selected="groupLabel">
          <li
            class="dropdown-option"
            v-for="option in groupOptions"
            :key="option.key"
            :class="{ selected: groupMode === option.key }"
            @click="selectGroup(option.key)">
            {{ option.label }}
          </li>
        </InstanceListDropdown>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import InstanceListDropdown from "./InstanceListDropdown.vue";
import { GroupMode, SortMode } from "./InstancesList.vue";

defineProps<{
  sortLabel: string;
  sortOptions: { key: SortMode; label: string }[];
  selectSort: (mode: SortMode) => void;
  groupLabel: string;
  groupOptions: { key: GroupMode; label: string }[];
  selectGroup: (mode: GroupMode) => void;
}>();
const sortMode = defineModel<SortMode>("sortMode", { required: true });
const groupMode = defineModel<GroupMode>("groupMode", { required: true });
</script>

<style lang="less" scoped>
.tool-bar {
  height: 112px;
  width: 352px;
  position: absolute;
  top: 8px;
  right: 280px;
  border-radius: 16px 0 0 16px;
  background: rgba(var(--ctp-surface0-rgb), 0.4);
  backdrop-filter: blur(4px);
  z-index: 114;

  .search {
    display: flex;
    width: 320px;
    height: 40px;
    margin-top: 16px;
    margin-left: 16px;
  }

  .search .search-input {
    background: rgba(var(--ctp-surface0-rgb), 1);
    border-radius: 8px 0 0 8px;
    width: 100%;

    input {
      appearance: none;
      border: none;
      background: none;
      font-size: 14px;
      height: 100%;
      padding-left: 16px;
    }
  }

  .search button.search-button {
    width: 40px;
    flex-shrink: 0;
    appearance: none;
    border: none;
    background: rgba(var(--ctp-surface1-rgb), 1);
    border-radius: 0 8px 8px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.1s ease;

    svg {
      transition: inherit;
    }

    &:hover {
      background: rgba(var(--ctp-surface2-rgb), 0.8);
    }

    &:active {
      background: rgba(var(--ctp-surface2-rgb), 1);

      svg {
        transform: scale(0.97);
      }
    }
  }

  .other {
    display: flex;
    width: 320px;
    margin-left: 16px;
    margin-top: 12px;

    > div {
      display: flex;
    }

    .sort {
      margin-right: 8px;
      flex-shrink: 0;
    }

    .group {
      width: 100%;
    }
  }
}
</style>
