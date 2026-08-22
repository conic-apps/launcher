<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.confirmDeleteSave.visible" :width="500">
    <div class="confirm-delete-save" ref="main">
      <div style="display: flex; align-items: center">
        <AppIcon name="warning" :size="50"></AppIcon>
        <div class="message">
          <p style="font-size: 17px">
            是否确认删除存档「{{ dialogStore.confirmDeleteSave.levelName }}」？
          </p>
          <p style="font-size: 12px; margin-top: 8px">存档文件夹将永久删除，最后的反悔机会</p>
        </div>
      </div>
      <div class="buttons">
        <button class="back" @click="dialogStore.confirmDeleteSave.visible = false">
          稍等一下...
        </button>
        <button class="quit" @click="confirmDelete" :disabled="deleting">确认删除</button>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseDialog from "@/components/BaseDialog.vue";
import { useDialogStore } from "@/store/dialog";
import { useGameContentStore } from "@/store/content";
import { useInstanceStore } from "@/store/instance";
import { deleteSave } from "@conic/content";
import { ref } from "vue";

const dialogStore = useDialogStore();
const instanceStore = useInstanceStore();
const gameContentStore = useGameContentStore();

const deleting = ref(false);

async function confirmDelete() {
  deleting.value = true;
  try {
    if (!instanceStore.currentInstance) {
      throw "CurrentInstance is null";
    }
    await deleteSave(instanceStore.currentInstance.id, dialogStore.confirmDeleteSave.folderName);
    await gameContentStore.refreshSaves();
    dialogStore.confirmDeleteSave.visible = false;
  } catch (error) {
    console.error("Delete save failed: ", error);
  } finally {
    deleting.value = false;
  }
}
</script>

<style lang="less" scoped>
.confirm-delete-save {
  padding: 8px;
  .message {
    display: flex;
    flex-direction: column;
    justify-content: center;
    margin-left: 16px;
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
