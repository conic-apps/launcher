<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-details">
    <div class="left-column">
      <p class="section-title">动态</p>
      <div v-for="(group, gIndex) in groupedActivity" :key="gIndex" class="date-group">
        <p class="date-label">{{ group.date }}</p>
        <div v-for="(item, index) in group.items" :key="index" class="activity-entry">
          <div class="activity-header">
            <img class="avatar" :src="item.avatar" alt="" />
            <span class="username">{{ item.username }} 游玩了存档</span>
          </div>
          <div class="activity-card">
            <img class="save-icon" :src="item.saveImage" alt="" />
            <div class="save-info">
              <p class="save-name">{{ item.saveName }}</p>
              <p class="save-desc">在该世界中游玩 {{ item.playtime }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
    <div class="right-column">
      <p class="section-title">内容</p>
      <div class="content-list">
        <div class="content-item">
          <AppIcon name="save" :size="15" />
          <span>存档</span>
          <span class="content-item-count">1</span>
        </div>
        <div class="content-item">
          <AppIcon name="extension-puzzle" :size="15" />
          <span>模组</span>
          <span class="content-item-count">1</span>
        </div>
        <div class="content-item">
          <AppIcon name="folder" :size="15" />
          <span>资源包</span>
          <span class="content-item-count">4</span>
        </div>
        <div class="content-item">
          <AppIcon name="aperture-outline" :size="15" />
          <span>光影包</span>
          <span class="content-item-count">5</span>
        </div>
        <div class="content-item">
          <AppIcon name="images-outline" :size="15" />
          <span>游戏截图</span>
          <span class="content-item-count">1</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import AppIcon from "@/components/AppIcon.vue";
import steveAvatar from "@/assets/images/steve_avatar.webp";

interface ActivityItem {
  avatar: string;
  username: string;
  saveName: string;
  saveImage: string;
  playtime: string;
  date: string;
}

const activities = ref<ActivityItem[]>([
  {
    avatar: steveAvatar,
    username: "Broken_Deer",
    saveName: "New World",
    saveImage: "/Unknown_server.webp",
    playtime: "3小时24分钟",
    date: "2024-03-15",
  },
  {
    avatar: steveAvatar,
    username: "Broken_Deer",
    saveName: "Survival Base",
    saveImage: "/Unknown_server.webp",
    playtime: "1小时05分钟",
    date: "2024-03-14",
  },
  {
    avatar: steveAvatar,
    username: "Broken_Deer",
    saveName: "Survival Base",
    saveImage: "/Unknown_server.webp",
    playtime: "2小时48分钟",
    date: "2024-03-13",
  },
  {
    avatar: steveAvatar,
    username: "Broken_Deer",
    saveName: "Creative Plot",
    saveImage: "/Unknown_server.webp",
    playtime: "0小时32分钟",
    date: "2024-03-12",
  },
  {
    avatar: steveAvatar,
    username: "Broken_Deer",
    saveName: "New World",
    saveImage: "/Unknown_server.webp",
    playtime: "5小时17分钟",
    date: "2024-03-10",
  },
]);

const groupedActivity = computed(() => {
  const groups: Record<string, ActivityItem[]> = {};
  for (const item of activities.value) {
    if (!groups[item.date]) {
      groups[item.date] = [];
    }
    groups[item.date].push(item);
  }
  return Object.entries(groups).map(([date, items]) => ({ date, items }));
});
</script>

<style lang="less" scoped>
.instance-details {
  display: flex;
  gap: 16px;
  width: 100%;
}

.left-column {
  flex: 1;
  min-width: 0;
}

.date-group {
  margin-bottom: 16px;

  .date-label {
    font-size: 13px;
    opacity: 0.5;
    margin-bottom: 8px;
  }
}

.activity-entry {
  margin-bottom: 8px;
}

.activity-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;

  .avatar {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
  }

  .username {
    font-size: 14px;
    opacity: 0.8;
  }
}

.activity-card {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  margin-left: 32px;
  background: var(--card-background);
  border: var(--card-border);
  border-radius: var(--card-border-radius);

  .save-icon {
    width: 40px;
    height: 40px;
    border-radius: var(--card-icon-border-radius);
    background: var(--card-icon-background);
    object-fit: cover;
    flex-shrink: 0;
  }

  .save-info {
    margin-left: 12px;
    min-width: 0;

    .save-name {
      font-size: 14px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .save-desc {
      font-size: 12px;
      opacity: 0.5;
      margin-top: 2px;
    }
  }
}

.right-column {
  width: 200px;
  flex-shrink: 0;

  .content-list {
    background: var(--card-background);
    border: var(--card-border);
    border-radius: var(--card-border-radius);
    overflow: hidden;
  }

  .content-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
    transition: background 0.15s ease;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;

    &:hover {
      background: var(--list-item-background);
    }

    &:not(:last-child) {
      border-bottom: var(--card-border);
    }

    .content-item-count {
      margin-left: auto;
      font-size: 13px;
      opacity: 0.5;
    }
  }
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 12px;
}
</style>
