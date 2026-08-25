<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="setup-wizard">
    <div class="header-wrapper">
      <div class="header">
        <p class="title">首次启动设置</p>
        <p class="description">调整 Conic Launcher 以更贴合你的习惯</p>
        <button class="close" @click="navigationStore.navigate('game')">
          <AppIcon name="xmark" :size="16"></AppIcon>
        </button>
      </div>
    </div>
    <div class="body">
      <ScrollView>
        <Transition :name="transitionName" mode="out-in">
          <SetupWizardLanguage v-if="currentPhase === 'language'" />
          <SetupWizardPalette v-else-if="currentPhase === 'palette'" />
          <SetupWizardAddAccount v-else-if="currentPhase === 'add-account'" />
          <SetupWizardJavaSettings v-else-if="currentPhase === 'java-settings'" />
          <SetupWizardLaunchOptions v-else-if="currentPhase === 'launch-options'" />
          <SetupWizardImportInstances v-else-if="currentPhase === 'import-instances'" />
        </Transition>
      </ScrollView>
    </div>
    <div class="footer">
      <button class="back" :class="{ disabled: currentPhaseIndex === 0 }" @click="backPhase">
        <AppIcon name="chevron-back" :size="20"></AppIcon>
        返回
      </button>
      <button class="next" @click="nextPhase">
        {{ nextPhaseTexts[currentPhaseIndex] }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import ScrollView from "@/components/ScrollView.vue";
import { useNavigationStore } from "@/store/navigation";
import { computed, ref } from "vue";
import SetupWizardAddAccount from "./setup/SetupWizardAddAccount.vue";
import SetupWizardImportInstances from "./setup/SetupWizardImportInstances.vue";
import SetupWizardJavaSettings from "./setup/SetupWizardJavaSettings.vue";
import SetupWizardLanguage from "./setup/SetupWizardLanguage.vue";
import SetupWizardLaunchOptions from "./setup/SetupWizardLaunchOptions.vue";
import SetupWizardPalette from "./setup/SetupWizardPalette.vue";

const navigationStore = useNavigationStore();

const transitionName = ref<"slide-left" | "slide-right">("slide-left");
const currentPhaseIndex = ref(0);
const phases = [
  "language",
  "palette",
  "add-account",
  "java-settings",
  "launch-options",
  "import-instances",
] as const;
const currentPhase = computed(() => phases[currentPhaseIndex.value]);
const nextPhaseTexts = [
  "让我们开始吧！",
  "下一个（添加帐户）",
  "下一个（Java 虚拟机设置）",
  "下一个（游戏启动选项）",
  "下一个（导入实例）",
  "完成",
];

function backPhase() {
  if (currentPhaseIndex.value > 0) {
    transitionName.value = "slide-right";
    currentPhaseIndex.value--;
  }
}

function nextPhase() {
  if (currentPhaseIndex.value < phases.length - 1) {
    transitionName.value = "slide-left";
    currentPhaseIndex.value++;
  } else {
    navigationStore.navigate("game");
  }
}
</script>

<style lang="less" scoped>
.setup-wizard {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;

  .header-wrapper {
    background: var(--ctp-base);
    border-radius: 0 0 12px 12px;
    width: calc(100% - 100px);
    height: fit-content;
    padding-bottom: 8px;
  }
  .header {
    background: rgba(var(--ctp-mauve-rgb), 0.2);
    border: 1px solid rgba(var(--ctp-mauve-rgb), 0.2);
    padding: 16px 32px;
    border-radius: 0 0 12px 12px;
    position: relative;
    .title {
      font-size: 16px;
    }
    .description {
      font-size: 12px;
      margin-top: 4px;
    }
    .close {
      position: absolute;
      right: 32px;
      top: 16px;
      appearance: none;
      border: none;
      background: none;
      padding: 2px;
      border-radius: 100px;
      transition: background 100ms ease;
      &:hover {
        background: rgba(var(--ctp-mauve-rgb), 0.4);
      }

      &:active {
        background: rgba(var(--ctp-mauve-rgb), 0.5);
      }
    }
  }
  .body {
    width: calc(100% - 186px);
    height: calc(100% - 176px);
    padding: 24px 36px;
    background: rgba(var(--ctp-base-rgb), 0.9);
    border-radius: 12px;
    margin-top: 16px;
    overflow: hidden;
    position: relative;

    :deep(.wizard-title) {
      color: var(--ctp-lavender);
      font-size: 22px;
      margin-bottom: 8px;
    }
    :deep(.wizard-message) {
      font-size: 13px;
      line-height: 1.5;
      width: 100%;
    }
    :deep(.wizard-message.warn) {
      border: 1px solid var(--ctp-yellow);
      border-radius: 4px;
      padding: 4px 8px;
      background: rgba(var(--ctp-yellow-rgb), 0.1);
    }
  }

  .footer {
    width: 100%;
    height: 48px;
    background: rgba(var(--ctp-surface0-rgb), 0.6);
    backdrop-filter: blur(4px);
    position: absolute;
    bottom: 0;
    left: 0;
    display: flex;
    gap: 32px;
    padding: 16px;
    button {
      font-size: 13px;
      margin-top: -24px;
      height: 40px;
      appearance: none;
      border: none;
      background: none;
      color: var(--ctp-text-inverse);
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 4px;
      &:hover {
        transition: none;
      }
      &,
      &:active {
        transition: background 100ms ease;
      }
    }
    button.disabled {
      opacity: 0.7;
      pointer-events: none;
    }
    .back {
      width: 200px;
      background-image: linear-gradient(var(--ctp-blue), var(--ctp-blue));
      background: var(--ctp-blue);
      svg :deep(path) {
        stroke: var(--ctp-text-inverse);
      }
      &:hover {
        background-image:
          linear-gradient(#ffffff2f, #ffffff2f), linear-gradient(var(--ctp-blue), var(--ctp-blue));
      }
      &:active {
        background-image:
          linear-gradient(#ffffff6f, #ffffff6f), linear-gradient(var(--ctp-blue), var(--ctp-blue));
      }
    }
    .next {
      flex: 1;
      background: var(--ctp-mauve);
      &:hover {
        background-image:
          linear-gradient(#ffffff2f, #ffffff2f), linear-gradient(var(--ctp-mauve), var(--ctp-mauve));
      }
      &:active {
        background-image:
          linear-gradient(#ffffff6f, #ffffff6f), linear-gradient(var(--ctp-mauve), var(--ctp-mauve));
      }
    }
  }
}
</style>
