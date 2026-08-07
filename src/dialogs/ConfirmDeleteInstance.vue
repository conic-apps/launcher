<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.confirmDeleteInstance.visible" :width="500">
    <div class="confirm-delete-instance" ref="main">
      <p style="font-size: 16px; margin-bottom: 16px; line-height: 1.3">
        是否确认删除此实例？实例下存档、资源包、Mod 等文件也将永久丢失 (真的很久！)
      </p>
      <div class="instance" style="margin-bottom: 16px">
        <p class="instance-name">
          {{ instanceToDelete.config.name }}
        </p>
        <div class="details">
          <span
            :class="`tag ${instanceToDelete.config.runtime.mod_loader_type.toLowerCase()}`"
            v-if="instanceToDelete.config.runtime.mod_loader_type"
            >{{ instanceToDelete.config.runtime.mod_loader_type }}</span
          >
          <span class="tag vanilla" v-else>Vanilla</span>
          <span class="minecraft-version"
            ><span class="label">Minecraft: </span
            ><span>{{ instanceToDelete.config.runtime.minecraft }}</span></span
          >
          <span class="playtime"
            ><span class="label">游戏时长：</span
            ><span>{{ formatPlayTime(playtime ?? 0) }}</span></span
          >
        </div>
        <img
          v-if="instanceToDelete.has_background"
          :src="backgroundFileSrc"
          alt=""
          class="background"
          v-show="imgLoaded"
          @load="imgLoaded = true"
          @error="imgLoaded = false" />
      </div>
      <div class="buttons">
        <BaseButton class="back" @click="dialogStore.confirmDeleteInstance.visible = false">
          取消
        </BaseButton>
        <!-- TODO: 长按确认 -->
        <BaseButton class="quit" @click="confirmDelete" :disabled="deleting">确认删除</BaseButton>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/BaseDialog.vue";
import BaseButton from "@/components/BaseButton.vue";
import { computed, ref, watch } from "vue";
import {
  calculatePlaytime,
  deleteInstance,
  formatPlayTime,
  getBackgroundPath,
} from "@conic/instance";
import { useDialogStore } from "@/store/dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useInstanceStore } from "@/store/instance";

const dialogStore = useDialogStore();
const instanceStore = useInstanceStore();
const instanceToDelete = computed(() => {
  return dialogStore.confirmDeleteInstance.instanceToDelete;
});

const deleting = ref(false);
const confirmDelete = async () => {
  deleting.value = true;
  try {
    await deleteInstance(instanceToDelete.value.id);
    await instanceStore.loadInstances();
    instanceStore.ensureCurrentInstanceAvailable();
    dialogStore.confirmDeleteInstance.visible = false;
  } catch (error) {
    console.error("Delete instance failed: ", error);
  } finally {
    deleting.value = false;
  }
};

const playtime = ref(null as null | number);
watch(
  instanceToDelete,
  async (newValue) => {
    const instanceId = newValue.id;
    try {
      playtime.value = await calculatePlaytime(instanceId);
    } catch (error) {
      console.error(error);
    }
  },
  { immediate: true },
);

const backgroundFileSrc = ref("");
const imgLoaded = ref(false);

watch(
  () => dialogStore.confirmDeleteInstance.instanceToDelete,
  async (instance) => {
    backgroundFileSrc.value = await getBackgroundSrc(instance.id);
  },
);
async function getBackgroundSrc(id: string) {
  const backgroundPath = await getBackgroundPath(id);
  return convertFileSrc(backgroundPath);
}
</script>

<style lang="less" scoped>
.confirm-delete-instance {
  width: 100%;
  height: 100%;
  padding: 12px;

  .instance {
    position: relative;
    border: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);
    border-left: 16px solid rgba(var(--ctp-lavender-rgb), 0.8);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    padding: 8px 16px;
    border-radius: 8px;
    height: 60px;
    transition:
      border-left 200ms ease,
      margin 200ms ease;
    img.background {
      mask-image: linear-gradient(to left, black 0%, transparent 100%);
      width: calc(100% - 100px);
      height: 100%;
      object-fit: cover;
      position: absolute;
      top: 0;
      right: 0;
      border-radius: 0 8px 8px 0;
    }
    p.instance-name {
      font-size: 16px;
      display: flex;
      align-items: center;
      height: 16px;
      span {
        max-width: calc(100% - 30px);
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
      }
      svg {
        margin-left: 4px;
        &:hover {
          transform: scale(1.02);
        }
        &:active {
          transform: scale(0.97);
        }
      }
      input {
        appearance: none;
        border: none;
        background: none;
        height: 100%;
        font-size: 16px;
        width: 100%;
      }
    }
    .details {
      margin-top: 6px;
      .tag {
        font-size: 11px;
        border-radius: 100px;
        padding: 1px 6px;
        font-weight: 500;
        display: inline-flex;
        align-items: center;
        width: fit-content;
      }
      .tag.quilt {
        background: var(--ctp-mauve);
        color: var(--ctp-text-inverse);
      }
      .tag.fabric {
        background: var(--ctp-yellow);
        color: var(--ctp-text-inverse);
      }
      .tag.forge {
        background: var(--ctp-blue);
        color: var(--ctp-text-inverse);
      }
      .tag.neoforge {
        background: var(--ctp-peach);
        color: var(--ctp-text-inverse);
      }
      .tag.vanilla {
        background: var(--ctp-green);
        color: var(--ctp-text-inverse);
      }
      .minecraft-version,
      .playtime {
        font-size: 11px;
        margin-left: 8px;
        font-weight: 500;
        .label {
          opacity: 0.8;
          font-weight: 300;
        }
      }
    }
  }

  .buttons {
    display: flex;
    width: 100%;
    margin-top: 16px;
    button {
      appearance: none;
      border: none;
      width: 100%;
      border-radius: 4px;
      transition: transform 200ms ease;
    }
    button.back {
      margin-right: 8px;
      background: var(--ctp-blue);
      color: var(--ctp-text-inverse);
      padding: 8px 0;
    }
    button.quit {
      background: var(--ctp-red);
      color: var(--ctp-text-inverse);
    }
    button:hover {
      transform: scale(1.02);
    }
    button:active {
      transform: scale(0.97);
    }
  }
}
</style>
