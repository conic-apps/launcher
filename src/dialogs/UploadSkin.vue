<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.uploadSkin.visible" :width="460" :height="300">
    <div class="upload-skin">
      <p
        style="
          width: 100%;
          margin-top: -4px;
          margin-bottom: 16px;
          padding-bottom: 16px;
          border-bottom: var(--card-border);
        ">
        {{ t("accounts.uploadSkin.title") }}
      </p>
      <div class="dialog-button" @click="close">
        <i></i>
      </div>
      <div class="body">
        <div class="preview">
          <SkinModel3D
            v-if="dialogStore.uploadSkin.textureType === 'skin'"
            :skin="dialogStore.uploadSkin.skinUrl"></SkinModel3D>
          <CapeView v-else :cape="dialogStore.uploadSkin.capeUrl"></CapeView>
        </div>
        <div class="right">
          <div class="section">
            <p class="section-title">{{ t("accounts.uploadSkin.modelType") }}</p>
            <div class="type-select">
              <div
                class="type-option"
                :class="{ activated: modelType === 'slim' }"
                @click="modelType = 'slim'">
                {{ t("accounts.uploadSkin.slim") }}
              </div>
              <div
                class="type-option"
                :class="{ activated: modelType === 'classic' }"
                @click="modelType = 'classic'">
                {{ t("accounts.uploadSkin.classic") }}
              </div>
            </div>
          </div>
          <div class="section" v-if="dialogStore.uploadSkin.accountType === 'Yggdrasil'">
            <p class="section-title">{{ t("accounts.uploadSkin.textureType") }}</p>
            <div class="type-select">
              <div
                class="type-option"
                :class="{ activated: dialogStore.uploadSkin.textureType === 'skin' }"
                @click="dialogStore.uploadSkin.textureType = 'skin'">
                {{ t("accounts.uploadSkin.skin") }}
              </div>
              <div
                class="type-option"
                :class="{ activated: dialogStore.uploadSkin.textureType === 'cape' }"
                @click="dialogStore.uploadSkin.textureType = 'cape'">
                {{ t("accounts.uploadSkin.cape") }}
              </div>
            </div>
          </div>
          <BaseButton style="width: 100%" @click="upload">{{
            t("accounts.uploadSkin.confirm")
          }}</BaseButton>
        </div>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/base/BaseDialog.vue";
import BaseButton from "@/components/base/BaseButton.vue";
import SkinModel3D from "@/components/SkinModel3D.vue";
import CapeView from "@/components/CapeView.vue";
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useDialogStore } from "@/store/dialog";

const { t } = useI18n();
const dialogStore = useDialogStore();

const modelType = ref<"slim" | "classic">("classic");

function close() {
  modelType.value = "classic";
  dialogStore.uploadSkin.textureType = "skin";
  dialogStore.uploadSkin.visible = false;
}

function upload() {
  // TODO: implement upload logic
}
</script>

<style lang="less" scoped>
.upload-skin {
  width: 100%;
  height: 100%;
  padding: 12px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;

  .body {
    width: 100%;
    display: flex;
    gap: 16px;
    flex: 1;
  }

  .preview {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .right {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-title {
    font-size: 13px;
    opacity: 0.7;
    margin: 0;
  }

  .type-select {
    display: flex;
    border: var(--controllers-border);
    background: var(--controllers-background);
    border-radius: var(--controllers-border-radius);
    overflow: hidden;

    .type-option {
      flex: 1;
      height: 32px;
      display: flex;
      justify-content: center;
      align-items: center;
      font-size: 13px;
      border-right: var(--controllers-border);
      transition: background-color 0.1s ease;
      cursor: pointer;

      &:last-child {
        border-right: none;
      }

      &.activated {
        background-color: rgba(var(--theme-color), 0.7);
      }

      &:active {
        opacity: 0.8;
      }
    }
  }
}

.dialog-button {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  position: absolute;
  top: 4px;
  right: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  background: var(--close-button-background);

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

.dialog-button:hover {
  i::before {
    opacity: 1;
  }
}

.dialog-button:active {
  i {
    opacity: 0.7;
  }
}
</style>
