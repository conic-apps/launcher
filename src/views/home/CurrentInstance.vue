<
<template>
  <div class="current-instance">
    <div class="left-column">
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
        <div class="tags">
          <p>
            <img :src="MinecraftLogo" />
            <span>
              {{ currentInstance.config.runtime.minecraft }}
            </span>
          </p>
          <p
            v-if="
              currentInstance.config.runtime.mod_loader_type &&
              currentInstance.config.runtime.mod_loader_version
            ">
            <img :src="modLoaderLogo" />
            <span>{{ currentInstance.config.runtime.mod_loader_version }}</span>
          </p>
        </div>
      </div>
    </div>
    <div class="right-column">
      <button class="game-button game-button-launch">
        <AppIcon name="play" style="margin-right: 4px" fill="#fff"></AppIcon>
        Launch
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import FabricLogo from "@/assets/images/fabric.webp";
import QuiltLogo from "@/assets/images/quilt.svg";
import NeoforgedLogo from "@/assets/images/neoforged.png";
import ForgeLogo from "@/assets/images/forge.svg";
import MinecraftLogo from "@/assets/images/minecraft.webp";
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
  display: flex;
  justify-content: space-between;
  background: var(--card-background);
  border: var(--card-border);
  border-radius: var(--card-border-radius);
  padding: 16px;
  transition: all 1s ease;

  .left-column {
    display: flex;

    .current-instance-icon {
      width: 52px;
      height: 52px;
      border-radius: calc(var(--card-icon-border-radius) + 4px);
      background: var(--card-icon-background);
    }
    .current-instance-info {
      margin-left: 16px;
      height: 100%;
      display: flex;
      flex-direction: column;
      justify-content: space-around;

      .title {
        font-size: 20px;
      }
      .tags {
        display: flex;
        p {
          font-size: 12px;
          display: flex;
          align-items: center;
          border-radius: 114514px;
          border: 1px solid #ffffff52;
          width: fit-content;
          padding: 2px 4px;
          margin-right: 6px;
          img {
            width: 14px;
            height: 14px;
            margin-right: 4px;
          }
          span {
            opacity: 0.8;
          }
        }
      }
    }
  }
  .right-column {
    display: flex;
    align-items: center;
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
    .game-button {
      appearance: none;
      background: none;
      border: none;
      color: #fff;
      padding: 8px 16px;
      font-size: 18px;
      display: flex;
      align-items: center;
      border-radius: var(--controllers-border-radius);
    }
    .game-button:active {
      opacity: 0.9;
    }
    .game-button-launch {
      background-color: var(--game-launch-button-background);
    }
  }
}
.current-instance-details {
  width: 100%;
  margin-top: 24px;
  height: 500px;
}
.instance-details-placeholder {
  width: 100%;
  height: calc(100% - 256px);
  border: 1px dashed #888;
  border-radius: var(--card-border-radius);
  margin-top: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  p {
    font-size: 20px;
    opacity: 0.6;
  }
}
</style>
