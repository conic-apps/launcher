<template>
  <li class="sidebar-item" :onclick="click">
    <i class="fa-regular nav-icon" :id="icon"></i>
    <span>{{ title }}</span>
  </li>
</template>

<script setup lang="ts">
defineProps<{
  title: string,
  icon: string,
  click?: string,
}>()
</script>

<style lang="less">
.sidebar-item {
  margin-bottom: 2px;
}
.sidebar-item i {
  pointer-events: none;
}
div.main-sidebar>div {
  margin: 16px 8px 7px 8px;
  height: calc(100% - 26px);
  display: flex;
  flex-direction: column;
}

div.main-sidebar ul.sidebar-links {
  display: flex;
  justify-content: space-between;
  flex-grow: 1;
  flex-direction: column;
}

div.main-sidebar ul.sidebar-links>div {
  width: 100%;
}

div.main-sidebar li:active {
  transform: scale(0.94);
}

div.main-sidebar li {
  transition: transform 0.17s ease;
  width: 100%;
  height: 36px;
  align-items: center;
  border-radius: 6px;
  white-space: nowrap;
  display: flex;
  padding-left: 7.5px;
}


div.main-sidebar li:hover {
  background-color: #aeaeae45;
}

div.main-sidebar .sidebar-links i {
  font-size: 1.1rem;
  font-weight: 400;
  font-family: "fa-pro";
  font-style: normal;
  width: 24px;
  display: inline-block;
  margin-right: 0.7rem;
  text-align: center;
  flex-shrink: 0;
  padding-left: 3px;
}

div.main-sidebar span {
  display: inline-block;
  white-space: nowrap;
  font-size: 14px;
  transition: all 0.2s ease;
}

.sidebar-button {
  width: 40px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--border-radius-medium);
  margin-left: 1.6px;
}

.sidebar-button:hover {
  background-color: #aeaeae45;
}

.sidebar-button:active {
  background-color: #90909045;
}

.sidebar-button i {
  font-size: 1rem;
  font-family: "fa-pro";
  font-style: normal;
  height: 1em;
  width: 1em;
  text-align: center;
  transition: all 0.2s cubic-bezier(0, 0.61, 0.16, 0.98);
}

.sidebar-button:active i {
  transform: scale(0.7, 1);
}

.sidebar-close {
  width: 60px !important;
  margin-right: -60px !important;
}

.sidebar-close span {
  opacity: 0;
}
</style>