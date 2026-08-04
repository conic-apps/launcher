<template>
  <div class="connect-extension-download-description">
    <div class="mode-unset">
      <div class="header">
        <p class="title">Conic Connect 跨局域网联机</p>
        <BaseButton>检查 NAT 类型</BaseButton>
      </div>
      <p class="description description-info">
        联机体验取决于你和其他参与者的网络环境。如果联机失败，请尝试改善 NAT 类型，或连接到有 IPV6
        的网络以提升联机成功率
      </p>
      <div class="actions">
        <button class="create-room">
          <AppIcon name="add-circle" :size="26"></AppIcon>
          <p>
            <span class="label">创建小组</span>
            <span class="description">创建小组并生成邀请码</span>
          </p>
        </button>
        <button class="enter">
          <AppIcon name="enter" :size="30"></AppIcon>
          <p>
            <span class="label">加入小组</span>
            <span class="description">通过邀请码加入小组后进入世界</span>
          </p>
        </button>
      </div>
      <div class="buttons">
        <BaseButton class="back" @click="dialogStore.connectExtension.visible = false">
          隐藏（此操作不会断开连接）
        </BaseButton>
      </div>
    </div>
    <div class="mode-host"></div>
    <div class="mode-guest"></div>
  </div>
</template>

<script setup lang="ts">
import { useDialogStore } from "@/store/dialog";
import BaseButton from "@/components/base/BaseButton.vue";
import AppIcon from "@/components/AppIcon.vue";

const dialogStore = useDialogStore();
</script>

<style lang="less" scoped>
.connect-extension-download-description {
  display: flex;
  flex-direction: column;
  height: 100%;
  .header {
    display: flex;
    button {
      width: fit-content;
      align-items: center;
    }
    p {
      display: flex;
      align-items: center;
    }
    margin-bottom: 16px;
  }
  .actions {
    display: flex;
    height: fit-content;
    gap: 8px;
    button {
      appearance: none;
      border: none;
      border-radius: 8px;
      width: 100%;
      height: 100%;
      background: var(--ctp-surface0);
      display: flex;
      align-items: center;
      font-size: 15px;
      padding: 12px 16px;
      transition: all 150ms ease;
      svg {
        width: 30px;
      }
      p {
        display: flex;
        flex-direction: column;
        align-items: start;
        gap: 4px;
        margin-left: 8px;
        span.label {
          font-size: 15px;
        }
        span.description {
          font-size: 12px;
          opacity: 0.8;
        }
      }
      &:hover {
        background: rgba(var(--ctp-surface2-rgb), 0.85);
        transform: scale(1.01);
      }

      &:active {
        background: var(--ctp-surface2);
        transform: scale(0.98);
      }
    }
  }
  .title {
    font-size: 22px;
  }
  p.description {
    font-size: 14px;
    line-height: 1.5;
    margin-bottom: 16px;
  }
  p.description-info {
    color: var(--ctp-blue);
    padding: 8px 16px;
    border: 1px solid var(--ctp-blue);
    border-radius: 8px;
    background: rgba(var(--ctp-blue-rgb), 0.15);
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
