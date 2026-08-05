<template>
  <div class="connect-extension-manager">
    <div class="header">
      <p class="title">Conic Connect 跨局域网联机</p>
      <BaseButton>检查 NAT 类型</BaseButton>
    </div>
    <div class="waiting" v-if="dialogStore.connectExtension.connectManagerComponent === 'waiting'">
      <p class="description description-info">
        联机体验取决于你和其他参与者的网络环境。如果联机失败，请尝试改善 NAT 类型，或连接到有 IPV6
        的网络以提升联机成功率
      </p>
      <div class="waiting-actions">
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
    </div>
    <div
      class="host-scan"
      v-else-if="dialogStore.connectExtension.connectManagerComponent === 'hostScan'">
      <p class="description description-info">
        请启动游戏，打开单人存档，按下 ESC 键，选择「对局域网开放」<br />
        当扫描到 Minecraft 世界后，房间码将会显示
      </p>
      <div style="display: flex" class="host-scan-progress">
        <div style="width: 100%">
          <p>扫描局域网世界...</p>
          <BaseProgress :value="0" :max="1" :loading="true"></BaseProgress>
        </div>
        <BaseButton class="back" style="width: fit-content; flex-shrink: 0; margin-left: 16px">
          取消
        </BaseButton>
      </div>
    </div>
    <div
      class="host-ready"
      v-else-if="dialogStore.connectExtension.connectManagerComponent === 'hostReady'"
      style="display: flex; flex-direction: column; height: 100%; padding-bottom: 16px">
      <p class="description description-info">
        请把下面的邀请码发送给好友，并提醒他们在启动器中输入邀请码加入小组
      </p>
      <div style="display: flex; height: calc(100% - 80px); gap: 16px">
        <div class="group-actions">
          <div style="display: flex; flex-direction: column; align-items: center">
            <p style="font-size: 13px; margin-top: 6px">邀请码（点击复制）</p>
            <Transition name="fade" mode="out-in">
              <p style="font-size: 16px; margin-top: 12px" v-if="showCopyMessage">已复制！</p>
              <p style="font-size: 16px; margin-top: 12px" v-else @click="copyCode">
                {{ hostCode }}
              </p>
            </Transition>
          </div>
          <BaseButton>关闭连接</BaseButton>
        </div>
        <div class="group-guests-list" style="overflow: auto">
          <div>
            <!-- NOTE: 如果不是房主，则不显示 “主机”的tag -->
            <p class="name">OakChaser <span class="tag">主机</span></p>
            <p class="env">Conic Nexus, Easytier v2.6.4</p>
          </div>
          <p class="message">等待其他玩家加入...</p>
        </div>
      </div>
    </div>
    <div
      class="guest-input-code"
      v-else-if="dialogStore.connectExtension.connectManagerComponent === 'guestCodeInput'">
      <div style="background: var(--ctp-surface0); border-radius: 8px; padding: 16px">
        <p class="description" style="text-align: center">输入好友发给你的邀请码以连接</p>
        <BaseInput style="width: 100%; text-align: center"></BaseInput>
      </div>
    </div>
    <div
      class="guest-joining"
      v-else-if="dialogStore.connectExtension.connectManagerComponent === 'guestJoining'">
      <div style="display: flex" class="host-scan-progress">
        <div style="width: 100%">
          <p>尝试加入小组...</p>
          <BaseProgress :value="0" :max="1" :loading="true"></BaseProgress>
        </div>
        <BaseButton class="back" style="width: fit-content; flex-shrink: 0; margin-left: 16px">
          取消
        </BaseButton>
      </div>
    </div>
    <div
      class="guest-ready"
      v-else-if="dialogStore.connectExtension.connectManagerComponent === 'guestReady'"
      style="height: calc(100% - 72px); padding-bottom: 16px">
      <div
        class="group-guests-list"
        style="width: 100%; height: 100%; justify-content: space-between">
        <div
          style="
            display: flex;
            flex-direction: column;
            gap: 12px;
            overflow: auto;
            max-height: calc(100% - 32px);
          ">
          <div>
            <!-- NOTE: 如果不是房主，则不显示 “主机”的tag -->
            <p class="name">OakChaser <span class="tag">主机</span></p>
            <p class="env">Conic Nexus, Easytier v2.6.4</p>
          </div>
        </div>
        <BaseButton>退出小组</BaseButton>
      </div>
    </div>
    <div class="buttons">
      <BaseButton class="quit" @click="dialogStore.connectExtension.visible = false">
        {{
          dialogStore.connectExtension.connectManagerComponent === "waiting"
            ? "关闭"
            : "隐藏窗口（不会断开连接）"
        }}
      </BaseButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useDialogStore } from "@/store/dialog";
import AppIcon from "@/components/AppIcon.vue";
import BaseProgress from "@/components/base/BaseProgress.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import { ref } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import BaseInput from "@/components/base/BaseInput.vue";

const dialogStore = useDialogStore();

const showCopyMessage = ref(false);
const hostCode = ref("U/2UGE-ZYTZ-1DZF-UW9S");

async function copyCode() {
  await writeText(hostCode.value);
  showCopyMessage.value = true;
  setTimeout(() => {
    showCopyMessage.value = false;
  }, 1500);
}
</script>

<style lang="less" scoped>
.connect-extension-manager {
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
  .waiting-actions {
    display: flex;
    height: fit-content;
    gap: 12px;
    button {
      appearance: none;
      border-radius: 8px;
      width: 100%;
      height: 100%;
      background: var(--ctp-surface0);
      border: var(--controllers-border);
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
    margin-top: auto;
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
  .host-scan-progress {
    padding: 16px;
    background: var(--ctp-surface0);
    border-radius: 8px;
    p {
      font-size: 14px;
      margin-bottom: 10px;
    }
  }
  .group-actions {
    width: 48%;
    height: 100%;
    background: var(--ctp-surface0);
    border-radius: 8px;
    padding: 12px;
    font-size: 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: space-between;
  }
  .group-guests-list {
    width: 52%;
    height: 100%;
    background: var(--ctp-surface0);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    p {
      display: flex;
      align-items: center;
    }
    p.name {
      font-size: 14px;
    }
    p.env {
      font-size: 12px;
      padding-top: 4px;
      opacity: 0.8;
    }
    p.message {
      font-size: 12px;
      text-align: center;
      justify-content: center;
      opacity: 0.6;
      font-style: italic;
      margin-top: 8px;
    }
    span.tag {
      background: rgba(var(--ctp-blue-rgb), 0.2);
      color: var(--ctp-blue);
      font-size: 11px;
      padding: 2px 4px;
      border-radius: 4px;
      margin-left: 4px;
    }
  }
}
</style>
