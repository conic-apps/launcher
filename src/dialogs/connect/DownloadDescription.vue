<template>
  <div class="connect-extension-download-description">
    <p class="title">下载 Conic Nexus 扩展</p>
    <p class="description">
      Conic Nexus 扩展基于 Easytier 构建，且实现了 Scaffolding 协议，可以与其他包括 HMCL
      在内的很多启动器联机
    </p>
    <p class="description">
      Conic Nexus 只使用 P2P
      连接，联机成功后房间内用户之间将直接连接，不使用第三方服务器转发您的流量，联机体验取决于参与联机者的网络情况。
    </p>
    <p class="description description-important">
      请注意，在多人联机全过程中，你必须严格遵守所在国家与地区的全部法律法规。
    </p>
    <div class="buttons">
      <BaseButton class="back" @click="dialogStore.connectExtension.visible = false">
        取消
      </BaseButton>
      <BaseButton
        class="start"
        @click="dialogStore.connectExtension.currentComponent = 'downloadProgress'"
        >开始下载</BaseButton
      >
    </div>
  </div>
</template>

<script setup lang="ts">
import { useDialogStore } from "@/store/dialog";
import BaseButton from "@/components/base/BaseButton.vue";

const dialogStore = useDialogStore();
defineEmits(["", "confirm-download"]);
</script>

<style lang="less" scoped>
.connect-extension-download-description {
  .title {
    font-size: 22px;
    text-align: center;
    margin-bottom: 16px;
  }
  .description {
    font-size: 14px;
    line-height: 1.5;
    margin-bottom: 8px;
  }
  .description-important {
    color: var(--ctp-red);
    padding: 8px 16px;
    border: 1px solid var(--ctp-red);
    border-radius: 8px;
    background: rgba(var(--ctp-red-rgb), 0.15);
  }
  div.buttons {
    margin-top: 16px;
    display: flex;
    gap: 12px;
    .start {
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
    }
    .stop {
      color: var(--ctp-red);
      border: 1px solid rgba(var(--ctp-red-rgb), 0.6);

      &:hover {
        background: var(--ctp-red);
        color: var(--ctp-text-inverse);
      }
    }
  }
}
</style>
