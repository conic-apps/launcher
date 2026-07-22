<template>
  <div class="current-instance">
    <div class="row-1">
      <img
        class="current-instance-icon"
        :src="currentInstance.config.icon ?? instanceIconFallback"
        alt="Instance icon" />
      <div class="current-instance-info">
        <!-- TODO: Click here to change Minecraft version -->
        <p class="title" v-if="currentInstance.id === LATEST_RELEASE_INSTANCE_ID">
          {{ $t("game.latestRelease") }}
        </p>
        <p class="title" v-else-if="currentInstance.id === LATEST_SNAPSHOT_INSTANCE_ID">
          {{ $t("game.latestSnapshot") }}
        </p>
        <p class="title" v-else>{{ currentInstance.config.name }}</p>
        <div class="env">
          <p>
            <span> Minecraft 版本 </span>
            <span>{{ currentInstance.config.runtime.minecraft }}</span>
          </p>
          <div
            class="line"
            v-if="
              currentInstance.config.runtime.mod_loader_type &&
              currentInstance.config.runtime.mod_loader_version
            "></div>
          <p
            v-if="
              currentInstance.config.runtime.mod_loader_type &&
              currentInstance.config.runtime.mod_loader_version
            ">
            <span> {{ currentInstance.config.runtime.mod_loader_type }} 版本 </span>
            <span>{{ currentInstance.config.runtime.mod_loader_version }}</span>
          </p>
        </div>
      </div>
    </div>
    <div class="row-2">
      <button class="launch-button">
        <AppIcon name="play" fill="#fff" style="margin-right: 4px"></AppIcon>
        开始游戏
      </button>
      <button class="launch-sub-button">
        <AppIcon
          name="chevron-down"
          stroke="#ffffff"
          fill="#ffffff"
          style="color: #fff"
          :size="16"></AppIcon>
      </button>
      <div class="play-data">
        <p>
          <span>最后运行日期</span>
          <span>昨天</span>
        </p>
        <div class="line"></div>
        <AppIcon name="time" :size="22" style="margin-right: 2px"></AppIcon>
        <p>
          <span>游戏时间</span>
          <span>1032.2 小时</span>
        </p>
      </div>
      <div class="actions">
        <button class="action-button">
          <AppIcon name="folder"></AppIcon>
        </button>
        <button class="action-button">
          <AppIcon name="share-social-outline"></AppIcon>
        </button>
        <button class="action-button">
          <AppIcon name="settings"></AppIcon>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import FabricLogo from "@/assets/images/fabric.webp";
import QuiltLogo from "@/assets/images/quilt.svg";
import NeoforgedLogo from "@/assets/images/neoforged.png";
import ForgeLogo from "@/assets/images/forge.svg";
import instanceIconFallback from "@/assets/images/Unknown_server.webp";
import { useInstanceStore } from "@/store/instance";
import { computed } from "vue";
import { LATEST_RELEASE_INSTANCE_ID, LATEST_SNAPSHOT_INSTANCE_ID } from "@conic/instance";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});
const modLoaderLogo = computed(() => {
  switch (currentInstance.value.config.runtime.mod_loader_type) {
    case "Quilt":
      return QuiltLogo;
    case "Fabric":
      return FabricLogo;
    case "Neoforged":
      return NeoforgedLogo;
    case "Forge":
      return ForgeLogo;
  }
});
</script>

<style lang="less" scoped>
.current-instance {
  width: 100%;

  .row-1 {
    display: flex;
    align-items: center;

    .current-instance-icon {
      width: 40px;
      height: 40px;
      border-radius: calc(var(--card-icon-border-radius) + 4px);
      background: var(--card-icon-background);
    }
    .current-instance-info {
      padding-left: 16px;
      width: 100%;
      height: 100%;
      display: flex;

      .title {
        font-size: 28px;
      }
    }
  }
  .row-2 {
    display: flex;
    align-items: center;
    margin-top: 16px;
    .open-instance-setting-button {
      appearance: none;
      background: none;
      border: none;
      width: 32px;
      height: 32px;
      margin-right: 16px;
    }
    .open-instance-setting-button:active {
      opacity: 0.9;
      transform: scale(0.95);
    }
    div.actions {
      display: flex;
      margin-left: auto;
    }
    .action-button {
      appearance: none;
      border: none;
      color: var(--ctp-text);
      width: 32px;
      height: 32px;
      border-radius: 100px;
      display: flex;
      align-items: center;
      justify-content: center;
      margin-right: 8px;
      background: none;
      transition:
        background 100ms ease,
        transform 100ms ease;
      &:hover {
        background: var(--ctp-surface1);
      }
      &:active {
        transform: scale(0.9);
        background: var(--ctp-surface0);
      }
      &:last-child {
        margin-right: 0;
      }
    }
    .launch-button {
      appearance: none;
      border: none;
      color: #fff;
      width: 114px;
      height: 36px;
      font-size: 14px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 8px 0 0 8px;
      background: rgb(114, 135, 253);
    }
    .launch-sub-button {
      appearance: none;
      border: none;
      background: #ffffff4f;
      height: 36px;
      width: 24px;
      border-radius: 0 8px 8px 0;
      background: rgb(114, 135, 253);
      margin-left: 2px;
    }
    .launch-button:active {
      opacity: 0.9;
    }
  }
  .env,
  .play-data {
    margin-left: auto;
    display: flex;
    align-items: center;
    &:active {
      opacity: 0.8;
    }
    > p {
      font-size: 12px;
      display: flex;
      flex-direction: column;
      align-items: initial;
      width: fit-content;
      padding: 2px 4px;
      :first-child {
        opacity: 0.8;
        font-size: 12px;
      }
      :last-child {
        margin-top: 2px;
        font-size: 15px;
      }
    }
    div.line {
      width: 1px;
      height: 26px;
      background: var(--ctp-surface2);
      margin: 0px 8px;
    }
  }
  .play-data {
    margin-left: 16px;
  }
}
</style>
