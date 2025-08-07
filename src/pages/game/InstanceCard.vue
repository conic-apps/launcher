<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-card" :class="expand ? '' : 'instance-card-unexpand'" :style="banner">
    <div
      v-if="expand"
      style="
        position: absolute;
        width: 100%;
        height: 100%;
        top: 0;
        left: 0;
        background-image: linear-gradient(
          rgba(0, 0, 0, 0.824),
          rgba(0, 0, 0, 0),
          rgba(0, 0, 0, 0.882)
        );
      "></div>
    <div class="line-a">
      <div style="display: flex">
        <div class="minecraft-version">
          <img src="@/assets/images/minecraft.webp" fill="#fff" />Minecraft
          {{ minecraftVersion }}
        </div>
        <div class="mod-loader-version" v-if="modLoaderType && modLoaderVersion">
          <img src="@/assets/images/fabric.webp" fill="#fff" v-if="modLoaderType === 'Fabric'" />
          <img src="@/assets/images/quilt.svg" fill="#fff" v-if="modLoaderType === 'Quilt'" />
          <img
            src="@/assets/images/neoforged.png"
            fill="#fff"
            v-if="modLoaderType === 'Neoforged'" />
          <img
            src="@/assets/images/Anvil_JE3_BE3.webp"
            fill="#fff"
            v-if="modLoaderType === 'Forge'" />
          {{ modLoaderType }} {{ modLoaderVersion }}
        </div>
      </div>
      <div class="expand-this-card" @click="expand = !expand">
        <i v-if="expand" class="arrows-to-dotted-line"></i>
        <i v-else class="arrows-from-dotted-line"></i>
      </div>
    </div>
    <div class="line-b">
      <div class="instance-name">
        {{ computedInstanceName }}
      </div>
      <div class="controll-btns">
        <i class="button gear"></i>
        <i class="button circle-info"></i>
        <i class="button star" id="star" @click="star"></i>
        <button
          class="game-button"
          :class="`${gameButtonType}-game-button`"
          @click="$emit(gameButtonType)"
          v-if="!buttonLoading">
          <i
            :class="gameButtonType"
            style="font-family: fa-pro; font-style: normal; margin-right: 5px; font-weight: 100"></i
          >{{ gameButtonText }}
        </button>
        <button
          class="game-button loading"
          :class="`${gameButtonType}-game-button`"
          v-if="buttonLoading">
          <item-loading-icon status="in-progress"></item-loading-icon>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import $ from "jquery";
import ItemLoadingIcon from "@/components/ItemLoadingIcon.vue";
import { useConfigStore } from "@/store/config";
import { useInstanceStore } from "@/store/instance";

const i18n = useI18n();
const config = useConfigStore();

const expand = ref(true);

defineProps<{
  buttonLoading: boolean;
  errorType?: "install" | "launch";
}>();

const instanceStore = useInstanceStore();

const currentInstance = computed(() => {
  return instanceStore.currentInstance;
});

const minecraftVersion = computed(() => {
  return instanceStore.currentInstance.config.runtime.minecraft;
});
const modLoaderType = computed(() => {
  return instanceStore.currentInstance.config.runtime.mod_loader_type;
});
const modLoaderVersion = computed(() => {
  return instanceStore.currentInstance.config.runtime.mod_loader_version;
});

const gameButtonType = computed(() => {
  if (instanceStore.currentInstance.installed) {
    return "launch";
  } else {
    return "install";
  }
});

const computedInstanceName = computed(() => {
  const name = currentInstance.value.config.name;
  if (name == "Latest Release") {
    return i18n.t("game.latestRelease");
  }
  if (name == "Latest Snapshot") {
    return i18n.t("game.latestSnapshot");
  }
  return name;
});

const banner = "";
const gameButtonText = computed(() => {
  switch (gameButtonType.value) {
    case "install":
      return i18n.t("game.install");
    case "launch":
      if (config.launch.is_demo) {
        return i18n.t("game.launchDemo");
      } else {
        return i18n.t("game.launch");
      }
    default:
      return "";
  }
});

function star() {
  const star = $("#star");
  if (star.hasClass("activated")) {
    star.removeClass("activated");
    return;
  }
  star.addClass("activated");
  star.attr("style", "transform: scale(1.18)");
  setTimeout(() => {
    star.removeAttr("style");
  }, 100);
}
</script>

<style lang="less" scoped>
.instance-card {
  width: 100%;
  height: 220px;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  background-image: url("@/assets/images/Java_Launcher_legacy_background.webp");
  background-position: center;
  background-repeat: none;
  background-size: cover;
  filter: brightness(0.94);
  justify-content: space-between;
  outline: var(--card-border);
  position: relative;
  overflow: hidden;
  border-radius: var(--card-border-radius);
}

.instance-card-unexpand {
  height: 76px;

  background: var(--card-background);

  .line-a {
    display: none;
  }
}

.minecraft-version,
.mod-loader-version,
.launch-progress,
.expand-this-card {
  width: fit-content;
  height: 32px;
  display: flex;
  align-items: center;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.09);
  padding: 16px 10px;
  border-radius: 6px;
  font-size: 14px;
}

.mod-loader-version {
  margin-left: 8px;
}

.minecraft-version img,
.mod-loader-version img {
  width: 22px;
  margin-right: 6px;
}

.expand-this-card i {
  font-family: "fa-pro";
  font-style: normal;
}

.instance-name {
  font-size: 22px;
  display: flex;
  align-items: center;
}

.line-a,
.line-b {
  z-index: 10;
}

.line-a {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.line-b {
  display: flex;
  justify-content: space-between;
}

.button {
  font-style: normal;
  font-family: "fa-pro";
  font-weight: 400;
  font-size: 18px;
  border-radius: 100px;
  margin: 0 2px;
  width: 24px;
  height: 24px;
  margin-right: 16px;
  display: inline-flex;
  transition: all 100ms ease-in-out;
  align-items: center;
  justify-content: center;
  opacity: 0.8;
  background-color: rgba(255, 255, 255, 0);
}

i.button::before {
  text-align: center;
}

i.button:hover {
  opacity: 1;
}

i.star:hover {
  color: rgba(227, 179, 65);
}

i.activated {
  opacity: 1 !important;
}

i.activated::before {
  color: rgba(227, 179, 65);
  font-weight: 100;
}

i.button:active {
  opacity: 0.86;
  transform: scale(0.9);
}

div.controll-btns {
  display: flex;
  align-items: center;
}

button.game-button {
  border: none;
  width: 120px;
  height: 36px;
  margin-left: 8px;
  border-radius: 8px;
  font-size: 14px;
  padding: 8px 10px;
  color: rgb(var(--default-text-color));
  // cursor: pointer;
  transition: all 100ms ease;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background-image: linear-gradient(248deg, #189e47, #41a126);
  transition: all 0.1s ease;

  i.launch::before {
    content: "\f04b";
  }

  i.install::before {
    content: "\f019";
  }
}

button.game-button:active {
  opacity: 0.8;
}

button.error-game-button {
  background-image: linear-gradient(248deg, #d11919, #d62f2f);
}

button.install-game-button {
  background-image: linear-gradient(248deg, #235dce, #399bed);
}

button.loading {
  pointer-events: none;
}
</style>
