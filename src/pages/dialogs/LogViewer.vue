<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <dialog-vue :visible="props.visible" :width="860" :height="520">
    <div class="log-viewer">
      <div class="title">
        <div style="display: flex; align-items: center">
          <div class="icon">
            <i class="scroll"></i>
          </div>
          <div>
            <h4>日志查看器</h4>
            <p style="display: flex; align-items: center">正在查看 {{ instanceName }} 的日志</p>
          </div>
          <div class="button" style="position: absolute; right: 0" @click="$emit('close')">
            <i></i>
          </div>
        </div>
      </div>
      <div class="logs">
        <transition-group>
          <div class="log" v-for="(log, index) in logCollector.get(instanceName)" :key="index">
            <div v-if="isNormal(log)">
              <span class="time">{{ getTime(log) }}</span>
              <span :class="getLevelClass(getLevel(log) as string)">{{ getLevel(log) }}</span>
              <span class="content">{{ getContent(log) }}</span>
            </div>
            <div v-else>
              <span class="other">{{ log }}</span>
            </div>
          </div>
        </transition-group>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import { useInstanceStore } from "@/store/instance";
import { listen } from "@tauri-apps/api/event";
import { computed, ref } from "vue";

const props = defineProps<{
  visible: boolean;
}>();

const instanceStore = useInstanceStore();

const instanceName = computed(() => {
  return instanceStore.currentInstance.config.name;
});

type Log = {
  instanceName: string;
  content: string;
};
// log collector
let logCollector = ref(new Map());

listen("log", (event) => {
  let payload = event.payload as Log;
  let instanceLog = logCollector.value.get(payload.instanceName);
  if (typeof instanceLog == "undefined") {
    logCollector.value.set(payload.instanceName, [payload.content]);
    return;
  }
  let newValue = logCollector.value.get(payload.instanceName);
  if (payload.content.trim().length == 0) {
    return;
  }
  newValue.push(payload.content);
  logCollector.value.set(payload.instanceName, newValue);
});

function getTime(log: string) {
  const regex = /\[([^\]]+)\]/;
  const match = log.match(regex);
  const extractedContent = match ? match[1] : null;
  return extractedContent;
}

function getLevel(log: string) {
  const regex = /\[(.*?)\]\s+\[(.*?)\]/;
  const match = log.match(regex);
  const logLevel = match ? match[2] : null;
  return logLevel;
}

function getContent(log: string) {
  const regex = /\[(.*?)\]\s+\[(.*?)\]\s*:\s*(.*)/;
  const match = log.match(regex);
  const logContent = match ? match[3] : null;
  return logContent;
}

function getLevelClass(logLevel: string) {
  if (logLevel.includes("ERROR")) {
    return "error";
  } else if (logLevel.includes("WARN")) {
    return "warn";
  } else if (logLevel.includes("INFO")) {
    return "info";
  } else if (logLevel.includes("DEBUG")) {
    return "debug";
  } else {
    return "unknown";
  }
}

function isNormal(log: string) {
  return log.startsWith("[");
}
</script>

<style lang="less">
.log-viewer {
  width: 100%;
  height: 100%;
  position: relative;

  .title {
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);

    h4 {
      font-weight: normal;
      font-size: 22px;
      margin-bottom: 4px;
    }

    p {
      font-size: 14px;
      margin-top: 4px;
      opacity: 0.7;
      font-weight: normal;
    }

    i {
      font-size: 13.6px;
    }

    .icon {
      width: 80px;
      height: 80px;
      display: flex;
      align-items: center;
      justify-content: center;

      i {
        width: 100%;
        height: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
      }

      i::before {
        font-size: 32px;
        font-weight: 500;
        font-style: normal;
        font-family: "fa-pro";
      }
    }
  }

  .logs {
    width: 100%;
    padding: 10px;
    height: calc(100% - 92px);
    overflow-y: auto;

    .log {
      span {
        user-select: text;
        -webkit-user-select: text;
        font-family: "FiraCode Nerd Font", "Consolas", monospace;
        cursor: text;
      }

      padding: 2px;
      border-radius: 4px;
      font-size: 13px;

      span {
        margin-right: 4px;
        padding: 2px 4px;
        border-radius: 4px;
      }

      .time {
        background: rgba(0, 217, 255, 0.151);
        border: 1px solid rgba(0, 217, 255, 0.151);
      }

      .target {
        background: rgba(0, 255, 221, 0.151);
        border: 1px solid rgba(0, 255, 221, 0.151);
      }

      .info {
        background: rgba(21, 255, 0, 0.151);
        border: 1px solid rgba(21, 255, 0, 0.151);
      }

      .error {
        background: rgba(255, 0, 0, 0.151);
        border: 1px solid rgba(255, 0, 0, 0.151);
      }

      .warn {
        background: rgba(255, 255, 0, 0.151);
        border: 1px solid rgba(255, 255, 0, 0.151);
      }

      .debug {
        background: rgba(255, 0, 255, 0.151);
        border: 1px solid rgba(255, 0, 255, 0.151);
      }

      .unknown {
        background: rgba(255, 255, 255, 0.151);
        border: 1px solid rgba(255, 255, 255, 0.151);
      }

      .content {
        font-size: 14px;
        line-height: 1.8;
      }

      .other {
        font-size: 14px;
        line-height: 1.8;
        padding-left: 16px;
        font-style: italic;
        color: rgba(var(--default-text-color), 0.64);
      }
    }

    .log:hover {
      background: rgba(255, 255, 255, 0.08);
    }
  }

  .button {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    margin-left: 8px;
    top: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: transform 100ms;
    background: var(--close-btn-background);

    i::before {
      content: "\f00d";
      font-size: 12px;
      margin-top: 1px;
      margin-left: 0.6px;
      font-style: normal;
      font-family: "fa-pro";
      opacity: 0;
      transition: all 70ms ease;
    }

    i {
      transition: all 100ms ease;
    }
  }

  .button:hover {
    i::before {
      opacity: 1;
    }
  }

  .button:active {
    i {
      opacity: 0.7;
    }
  }
}
</style>
