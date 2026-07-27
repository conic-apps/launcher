<template>
  <div class="launch-view">
    <div class="container">
      <p class="title" v-if="currentInstance.installed">
        正在启动 {{ currentInstance.config.name }}
      </p>
      <p class="title" v-else>正在准备 {{ currentInstance.config.name }}</p>
      <div class="cards-container launch-only" v-if="currentInstance.installed">
        <div class="card"></div>
      </div>
      <div class="cards-container install-launch" v-else>
        <div class="card">
          <p class="title">解析版本信息</p>
          <p class="details">
            Conic Launcher 正在获取并解析此 Minecraft 版本的有关信息，以确定当前实例所需的文件。
          </p>
        </div>
        <div class="scrollline"></div>
        <div class="card">
          <p class="title">校验已有文件</p>
          <p class="details"></p>
        </div>
        <div class="scrollline"></div>
        <div class="card">
          <p class="title">下载新文件</p>
          <p class="details"></p>
        </div>
        <div
          class="scrollline"
          v-if="
            currentInstance.config.runtime.mod_loader_type &&
            currentInstance.config.runtime.mod_loader_version
          "></div>
        <div
          class="card"
          v-if="
            currentInstance.config.runtime.mod_loader_type &&
            currentInstance.config.runtime.mod_loader_version
          ">
          <p class="title">安装 {{ currentInstance.config.runtime.mod_loader_type }}</p>
          <p class="details"></p>
        </div>
        <div class="card">
          <p class="title">启动游戏</p>
          <p class="details"></p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useInstanceStore } from "@/store/instance";
import { computed } from "vue";

const instanceStore = useInstanceStore();
const currentInstance = computed(() => instanceStore.currentInstance);
</script>

<style lang="less" scoped>
.launch-view {
  width: 100%;
  height: 100%;
  position: relative;
  .container {
    position: absolute;
    display: flex;
    flex-direction: column;
    top: calc(50% - 100px);
    gap: 16px;
    width: 100%;
    > .title {
      font-size: 20px;
      width: 100%;
      text-align: center;
    }
    .cards-container {
      width: 100%;
      display: flex;
      transform: translate(100px, 0);
      .card {
        width: 240px;
        height: 200px;
        background: var(--ctp-base);
        padding: 16px;
        border-radius: 8px;
        flex-shrink: 0;
        .title {
          font-size: 16px;
        }
        .details {
          font-size: 14px;
        }
      }
      .scrollline {
        flex-shrink: 0;
        width: 100px;
      }
    }
  }
}
</style>
