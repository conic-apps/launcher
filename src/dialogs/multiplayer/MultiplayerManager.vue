<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="connect-extension-manager">
    <div class="header">
      <p class="title">Conic Nexus 跨局域网联机</p>
      <p class="nat" v-if="localNatCode !== null">你的网络环境：{{ localNatLabel }}</p>
    </div>
    <Transition mode="out-in" :name="transitionName">
      <div class="waiting" v-if="uiComponent === 'waiting'">
        <p class="description description-info">
          联机体验取决于你和其他参与者的网络环境。如果联机失败，请尝试改善 NAT 类型，或连接到有 IPV6
          的网络以提升联机成功率
        </p>
        <div class="waiting-actions">
          <button class="create-room" @click="createRoom">
            <AppIcon name="add-circle" :size="26"></AppIcon>
            <p>
              <span class="label">创建小组</span>
              <span class="description">创建小组并生成邀请码</span>
            </p>
          </button>
          <button class="enter" @click="openJoinCodeInput">
            <AppIcon name="enter" :size="30"></AppIcon>
            <p>
              <span class="label">加入小组</span>
              <span class="description">通过邀请码加入小组后进入世界</span>
            </p>
          </button>
        </div>
      </div>
      <div class="host-scan" v-else-if="uiComponent === 'hostScan'">
        <p class="description description-info">
          请启动游戏，打开单人存档，按下 ESC 键，选择「对局域网开放」<br />
          当扫描到 Minecraft 世界后，房间码将会显示
        </p>
        <div style="display: flex" class="host-scan-progress">
          <div style="width: 100%">
            <p>扫描局域网世界...</p>
            <BaseProgress :value="0" :max="1" :loading="true"></BaseProgress>
          </div>
          <BaseButton
            class="back"
            style="width: fit-content; flex-shrink: 0; margin-left: 16px"
            @click="leaveRoom">
            取消
          </BaseButton>
        </div>
      </div>
      <div
        class="host-ready"
        v-else-if="uiComponent === 'hostReady'"
        style="display: flex; flex-direction: column; height: 100%; padding-bottom: 16px">
        <p class="description description-info">
          请把下面的邀请码发送给好友，并提醒他们在启动器中输入邀请码加入小组
        </p>
        <div style="display: flex; height: calc(100% - 56px); gap: 16px">
          <div class="group-actions">
            <div style="display: flex; flex-direction: column; align-items: center">
              <p style="font-size: 13px; margin-top: 6px">邀请码（点击复制）</p>
              <Transition :name="showCopyMessage ? 'zoom-out' : 'zoom-in'" mode="out-in">
                <p style="font-size: 16px; margin-top: 12px" v-if="showCopyMessage">已复制！</p>
                <p style="font-size: 16px; margin-top: 12px" v-else @click="copyCode">
                  {{ multiplayerStore.roomCode }}
                </p>
              </Transition>
            </div>
            <BaseButton @click="leaveRoom">关闭连接</BaseButton>
          </div>
          <div class="group-guests-list" style="overflow: auto">
            <template v-for="player in multiplayerStore.players" :key="player.machine_id">
              <div>
                <p class="name">
                  {{ player.name }} <span class="tag" v-if="player.kind === 'HOST'">主机</span>
                </p>
                <p class="vendor">{{ player.vendor }}</p>
              </div>
            </template>
            <p class="message" v-if="multiplayerStore.players.length === 0">等待其他玩家加入...</p>
          </div>
        </div>
      </div>
      <div class="guest-input-code" v-else-if="uiComponent === 'guestCodeInput'">
        <div style="background: var(--ctp-surface0); border-radius: 8px; padding: 16px">
          <p class="description" style="text-align: center">输入好友发给你的邀请码以连接</p>
          <div style="display: flex; width: 100%; align-items: center; gap: 8px">
            <BaseInput
              v-model="codeInput"
              style="flex: 1"
              placeholder="邀请码格式：U/BBBB-AAAA-KKKK-AAAA"></BaseInput>
            <BaseButton
              style="flex-shrink: 0; width: fit-content"
              @click="submitJoin"
              :disabled="!codeInputValid"
              >加入</BaseButton
            >
          </div>
        </div>
      </div>
      <div class="guest-joining" v-else-if="uiComponent === 'guestJoining'">
        <div style="display: flex" class="host-scan-progress">
          <div style="width: 100%">
            <p>尝试加入小组...</p>
            <BaseProgress :value="0" :max="1" :loading="true"></BaseProgress>
          </div>
          <BaseButton
            class="back"
            style="width: fit-content; flex-shrink: 0; margin-left: 16px"
            @click="leaveRoom">
            取消
          </BaseButton>
        </div>
      </div>
      <div
        class="guest-ready"
        v-else-if="uiComponent === 'guestReady'"
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
            <template v-for="player in multiplayerStore.players" :key="player.machine_id">
              <div>
                <p class="name">
                  {{ player.name }} <span class="tag" v-if="player.kind === 'HOST'">主机</span>
                </p>
                <p class="vendor">{{ player.vendor }}</p>
              </div>
            </template>
          </div>
          <BaseButton @click="leaveRoom">退出小组</BaseButton>
        </div>
      </div>
      <div
        class="exception"
        v-else-if="uiComponent === 'exception'"
        style="display: flex; flex-direction: column; gap: 16px; height: 100%">
        <p
          class="description description-error"
          v-if="multiplayerStore.fault"
          style="display: flex; align-items: center">
          <span
            >连接出现问题（{{ multiplayerStore.fault.code }}）：{{
              multiplayerStore.fault.message
            }}</span
          >
          <BaseButton style="width: fit-content" @click="leaveRoom">重新开始</BaseButton>
        </p>
        <p class="description description-error" v-else>
          连接出现问题，请重新开始
          <BaseButton style="width: fit-content" @click="leaveRoom">重新开始</BaseButton>
        </p>
      </div>
    </Transition>
    <div
      class="buttons"
      v-if="dialogStore.multiplayerExtension.multiplayerManagerComponent === 'guestCodeInput'">
      <BaseButton
        class="back"
        @click="
          guestCodeInput = false;
          codeInput = '';
        ">
        返回
      </BaseButton>
    </div>
    <div class="buttons" v-else>
      <BaseButton class="quit" @click="dialogStore.multiplayerExtension.visible = false">
        {{
          uiComponent === "waiting" || uiComponent === "exception"
            ? "关闭"
            : "隐藏窗口（不会断开连接）"
        }}
      </BaseButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useDialogStore } from "@/store/dialog";
import { useMultiplayerStore } from "@/store/multiplayer";
import { useConfigStore } from "@/store/config";
import AppIcon from "@/components/AppIcon.vue";
import BaseProgress from "@/components/BaseProgress.vue";
import BaseButton from "@/components/BaseButton.vue";
import { computed, ref, watch } from "vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import BaseInput from "@/components/BaseInput.vue";
import { isRoomCodeValid } from "@conic/multiplayer";

const dialogStore = useDialogStore();
const multiplayerStore = useMultiplayerStore();
const configStore = useConfigStore();

const transitionName = ref("slide-left");

multiplayerStore.init();

const showCopyMessage = ref(false);
const guestCodeInput = ref(false);
const codeInput = ref("");

const codeInputValid = computed(() => {
  return isRoomCodeValid(codeInput.value);
});

const profileName = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.profile_name;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.name;
  } else {
    return configStore.current_account ? configStore.current_account.data.name : "";
  }
});

const uiComponent = computed(() => {
  if (guestCodeInput.value) {
    return "guestCodeInput";
  }
  switch (multiplayerStore.state) {
    case "waiting":
      return "waiting";
    case "host-scanning":
    case "host-starting":
      return "hostScan";
    case "host-ok":
      return "hostReady";
    case "guest-connecting":
    case "guest-starting":
      return "guestJoining";
    case "guest-ok":
      return "guestReady";
    case "exception":
      return "exception";
    default:
      return "waiting";
  }
});

const NAT_TYPE_NAMES: Record<number, string> = {
  0: "Unknown",
  1: "OpenInternet",
  2: "NoPAT",
  3: "FullCone",
  4: "Restricted",
  5: "PortRestricted",
  6: "Symmetric",
  7: "SymUdpFirewall",
  8: "SymmetricEasyInc",
  9: "SymmetricEasyDec",
};

const localPeer = computed(() => multiplayerStore.peers.find((peer) => peer.is_local));

const localNatCode = computed(() => localPeer.value?.nat ?? null);

const localNatLabel = computed(() => {
  const code = localNatCode.value;
  return code === null ? "--" : (NAT_TYPE_NAMES[code] ?? `NAT${code}`);
});

watch(
  uiComponent,
  (value) => {
    transitionName.value = value === "waiting" ? "slide-right" : "slide-left";
    dialogStore.multiplayerExtension.multiplayerManagerComponent = value;
  },
  { immediate: true },
);

async function copyCode() {
  await writeText(multiplayerStore.roomCode);
  showCopyMessage.value = true;
  setTimeout(() => {
    showCopyMessage.value = false;
  }, 1500);
}

function openJoinCodeInput() {
  guestCodeInput.value = true;
  codeInput.value = "";
}

async function createRoom() {
  try {
    await multiplayerStore.createRoom(profileName.value);
  } catch (error) {
    console.error(error);
  }
}

async function submitJoin() {
  const code = codeInput.value.trim();
  if (!code) return;
  try {
    await multiplayerStore.joinRoom(code, profileName.value);
    guestCodeInput.value = false;
  } catch (error) {
    console.error(error);
  }
}

async function leaveRoom() {
  try {
    await multiplayerStore.leaveRoom();
  } catch (error) {
    console.error(error);
  }
}
</script>

<style lang="less" scoped>
.connect-extension-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding-bottom: 48px;
  .header {
    display: flex;
    p.nat {
      width: fit-content;
      align-items: center;
      margin-left: auto;
      font-size: 14px;
      opacity: 0.7;
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
    padding: 4px 0;
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
  p.description-error {
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
    position: absolute;
    bottom: 24px;
    width: calc(100% - 48px);
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
    width: 44%;
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
    width: 56%;
    height: 100%;
    background: var(--ctp-surface0);
    border-radius: 8px;
    padding: 12px 16px;
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
    p.vendor {
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
