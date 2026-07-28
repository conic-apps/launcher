<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-view">
    <div class="sidebar">
      <ul class="category-list">
        <li
          v-for="cat in categories"
          :key="cat.key"
          :class="{ active: activeCategory === cat.key }"
          @click="activeCategory = cat.key">
          <AppIcon :name="cat.icon" :size="16" />
          <span>{{ cat.label }}</span>
        </li>
      </ul>
      <button class="back-button" @click="navigation.back()">
        <AppIcon name="arrow-back-outline" :size="16" />
        <span>返回</span>
      </button>
    </div>

    <div class="main-area">
      <div class="toolbar">
        <div class="search-box">
          <AppIcon name="search" :size="15" />
          <input v-model="searchQuery" type="text" placeholder="搜索" />
        </div>
        <div class="toolbar-actions">
          <div class="sort-group" ref="sortGroupRef">
            <button class="toolbar-btn" @click="sortDropdownOpen = !sortDropdownOpen">
              <AppIcon name="time" :size="16" />
              <span class="sort-label">{{ sortLabel }}</span>
              <AppIcon name="chevron-down" :size="12" />
            </button>
            <div class="sort-dropdown" v-if="sortDropdownOpen">
              <div
                v-for="opt in sortOptions"
                :key="opt.key"
                class="sort-option"
                :class="{ active: sortMode === opt.key }"
                @click="selectSort(opt.key)">
                {{ opt.label }}
              </div>
            </div>
          </div>
          <button
            class="toolbar-btn"
            :class="{ active: viewMode === 'grid' }"
            @click="viewMode = 'grid'"
            title="图标视图">
            <AppIcon name="apps-outline" :size="16" />
          </button>
          <button
            class="toolbar-btn"
            :class="{ active: viewMode === 'list' }"
            @click="viewMode = 'list'"
            title="列表视图">
            <AppIcon name="document-text" :size="16" />
          </button>
          <button class="toolbar-btn add-game-btn" @click="dialog.createInstance.visible = true">
            <AppIcon name="add" :size="16" />
            <span>添加游戏</span>
          </button>
        </div>
      </div>

      <div v-if="viewMode === 'grid'" class="instance-container grid">
        <div
          v-for="instance in filteredInstances"
          :key="instance.id"
          class="instance-card"
          :class="{ selected: instanceStore.currentInstance?.id === instance.id }"
          :style="instance.config.background ? { backgroundImage: `url(data:image/png;base64,${instance.config.background})` } : {}"
          @click="instanceStore.currentInstance = instance">
          <div class="instance-card-bg" v-if="instance.config.background" />
          <span class="instance-name">{{ instance.config.name }}</span>
          <AppIcon name="settings" :size="14" class="instance-settings-icon" />
        </div>
        <div v-if="filteredInstances.length === 0" class="empty-state">
          <AppIcon name="folder" :size="48" />
          <span>没有找到实例</span>
        </div>
      </div>

      <div v-else class="instance-container list">
        <div
          v-for="instance in filteredInstances"
          :key="instance.id"
          class="instance-card"
          :class="{ selected: instanceStore.currentInstance?.id === instance.id }"
          @click="instanceStore.currentInstance = instance">
          <div class="instance-icon">
            <img v-if="instance.config.icon" :src="instance.config.icon" alt="" />
            <AppIcon v-else name="gamepad" :size="20" />
          </div>
          <div class="instance-info">
            <span class="instance-name">{{ instance.config.name }}</span>
            <span class="instance-meta">
              {{ instance.config.runtime.minecraft }}
              <template v-if="instance.config.runtime.mod_loader_type">
                · {{ instance.config.runtime.mod_loader_type }}
              </template>
            </span>
          </div>
        </div>
        <div v-if="filteredInstances.length === 0" class="empty-state">
          <AppIcon name="folder" :size="48" />
          <span>没有找到实例</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue"
import { useInstanceStore } from "@/store/instance"
import { useNavigationStore } from "@/store/navigation"
import { useDialogStore } from "@/store/dialog"

const instanceStore = useInstanceStore()
const navigation = useNavigationStore()
const dialog = useDialogStore()

type CategoryKey = "all" | "vanilla" | "quilt" | "fabric" | "forge" | "neoforge" | "modpack"
type ViewMode = "grid" | "list"
type SortMode = "name" | "version"

const categories: { key: CategoryKey; label: string; icon: string }[] = [
  { key: "all", label: "全部游戏", icon: "gamepad" },
  { key: "vanilla", label: "原版", icon: "checkmark-outline" },
  { key: "quilt", label: "Quilt", icon: "branch" },
  { key: "fabric", label: "Fabric", icon: "extension-puzzle" },
  { key: "forge", label: "Forge", icon: "build" },
  { key: "neoforge", label: "Neoforge", icon: "contrast" },
  { key: "modpack", label: "整合包", icon: "package" },
]

const activeCategory = ref<CategoryKey>("all")
const searchQuery = ref("")
const viewMode = ref<ViewMode>("grid")
const sortMode = ref<SortMode>("name")
const sortDropdownOpen = ref(false)
const sortGroupRef = ref<HTMLDivElement>()

const sortOptions: { key: SortMode; label: string }[] = [
  { key: "name", label: "名称" },
  { key: "version", label: "版本" },
]

const sortLabel = computed(() => {
  return sortOptions.find((o) => o.key === sortMode.value)?.label ?? ""
})

function selectSort(mode: SortMode) {
  sortMode.value = mode
  sortDropdownOpen.value = false
}

function onClickOutside(e: MouseEvent) {
  if (sortGroupRef.value && !sortGroupRef.value.contains(e.target as Node)) {
    sortDropdownOpen.value = false
  }
}

onMounted(() => document.addEventListener("click", onClickOutside))
onBeforeUnmount(() => document.removeEventListener("click", onClickOutside))

function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number)
  const pb = b.split(".").map(Number)
  const len = Math.max(pa.length, pb.length)
  for (let i = 0; i < len; i++) {
    const na = pa[i] ?? 0
    const nb = pb[i] ?? 0
    if (na !== nb) return nb - na
  }
  return 0
}

const filteredInstances = computed(() => {
  let list = [...(instanceStore.instances ?? [])]

  if (activeCategory.value === "vanilla") {
    list = list.filter((i) => !i.config.runtime.mod_loader_type)
  } else if (activeCategory.value !== "all" && activeCategory.value !== "modpack") {
    const loaderMap: Record<string, string> = {
      quilt: "Quilt",
      fabric: "Fabric",
      forge: "Forge",
      neoforge: "Neoforge",
    }
    const target = loaderMap[activeCategory.value]
    if (target) {
      list = list.filter((i) => i.config.runtime.mod_loader_type === target)
    }
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.trim().toLowerCase()
    list = list.filter(
      (i) =>
        i.config.name.toLowerCase().includes(q) ||
        i.config.runtime.minecraft.toLowerCase().includes(q) ||
        (i.config.runtime.mod_loader_type && i.config.runtime.mod_loader_type.toLowerCase().includes(q)),
    )
  }

  list.sort((a, b) => {
    if (sortMode.value === "name") {
      return a.config.name.localeCompare(b.config.name)
    }
    if (sortMode.value === "version") {
      return compareVersions(a.config.runtime.minecraft, b.config.runtime.minecraft)
    }
    return 0
  })

  return list
})
</script>

<style lang="less" scoped>
.instances-view {
  width: 100%;
  height: 100%;
  display: flex;
  gap: 12px;
  padding: 0 12px 12px 12px;
  box-sizing: border-box;
}

.sidebar {
  width: 200px;
  flex-shrink: 0;
  background: var(--ctp-base);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 12px;
  box-sizing: border-box;
}

.category-list {
  list-style: none;
  margin: 0;
  padding: 0;

  li {
    height: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border-radius: 8px;
  
    font-size: 13px;
    color: var(--ctp-subtext1);
    transition: background 120ms ease, color 120ms ease;
    user-select: none;

    &:hover {
      background: var(--ctp-surface0);
      color: var(--ctp-text);
    }

    &.active {
      background: var(--ctp-surface1);
      color: var(--ctp-text);
    }

    span {
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
  }
}

.back-button {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--ctp-subtext1);
  font-size: 13px;

  transition: background 120ms ease, color 120ms ease;

  &:hover {
    background: var(--ctp-surface0);
    color: var(--ctp-text);
  }
}

.main-area {
  flex: 1;
  min-width: 0;
  background: var(--ctp-base);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  gap: 12px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--ctp-surface0);
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--ctp-surface0);
  border-radius: 8px;
  padding: 6px 12px;
  flex: 1;
  max-width: 320px;
  color: var(--ctp-subtext0);
  transition: background 150ms ease;

  &:focus-within {
    background: var(--ctp-surface1);
  }

  input {
    border: none;
    outline: none;
    background: transparent;
    color: var(--ctp-text);
    font-size: 13px;
    width: 100%;

    &::placeholder {
      color: var(--ctp-overlay1);
    }
  }
}

.toolbar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.toolbar-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--ctp-subtext0);
  font-size: 12px;

  transition: background 120ms ease, color 120ms ease;
  white-space: nowrap;

  &:hover {
    background: var(--ctp-surface0);
    color: var(--ctp-text);
  }

  &.active {
    background: var(--ctp-surface1);
    color: var(--ctp-text);
  }
}

.sort-group {
  margin-left: 4px;
  position: relative;
}

.add-game-btn {
  margin-left: 8px;
}

.sort-dropdown {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  min-width: 120px;
  background: var(--ctp-surface0);
  border: 1px solid var(--ctp-surface1);
  border-radius: 8px;
  padding: 4px;
  z-index: 10;
}

.sort-option {
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 12px;
  color: var(--ctp-subtext1);
  transition: background 100ms ease, color 100ms ease;

  &:hover {
    background: var(--ctp-surface1);
    color: var(--ctp-text);
  }

  &.active {
    color: var(--ctp-text);
    background: var(--ctp-surface1);
  }
}

.sort-label {
  font-size: 12px;
}

.instance-container {
  flex: 1;
  overflow-y: auto;
  padding: 16px;

  &.grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px;
    align-content: start;
  }

  &.list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
}

.instance-card {
  transition: transform 80ms ease;

  &:active {
    transform: scale(0.98);
  }
}

.grid .instance-card {
  display: flex;
  align-items: flex-end;
  position: relative;
  min-height: 120px;
  padding: 12px;
  border-radius: 10px;
  background-color: var(--ctp-surface0);
  background-size: cover;
  background-position: center;

  &:hover {
    background-color: var(--ctp-surface1);
  }

  &.selected {
    background-color: var(--ctp-surface1);
    outline: 2px solid var(--ctp-blue);
  }
}

.instance-settings-icon {
  position: absolute;
  bottom: 10px;
  right: 10px;
  flex-shrink: 0;
  color: var(--ctp-overlay1);
  z-index: 1;
}

.grid .instance-settings-icon {
  opacity: 1;
}

.instance-card-bg {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  background: linear-gradient(to top, rgba(var(--ctp-base-rgb), 0.85) 0%, rgba(var(--ctp-base-rgb), 0.3) 100%);
  pointer-events: none;
}

.list .instance-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 8px;

  &:hover {
    background: var(--ctp-surface0);
  }

  &.selected {
    background: var(--ctp-surface1);
  }
}

.instance-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  overflow: hidden;
  flex-shrink: 0;
  color: var(--ctp-overlay1);
  position: relative;
}

.list .instance-icon {
  width: 36px;
  height: 36px;
  background: var(--ctp-surface1);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}

.instance-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  position: relative;
}

.list .instance-info {
  flex: 1;
  min-width: 0;
}

.instance-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--ctp-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.instance-meta {
  font-size: 11px;
  color: var(--ctp-subtext0);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 48px 0;
  color: var(--ctp-overlay1);
  font-size: 14px;
}
</style>
