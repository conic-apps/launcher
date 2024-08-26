<template>
  <div class="instance-info card" :style="banner">
    <div
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
      <div>
        <div class="minecraft-version" v-if="minecraftVersion">
          <img src="@/assets/images/Grass_Block_JE2.webp" fill="#fff" />Minecraft
          {{ minecraftVersion }}
        </div>
        <div class="mod-loader-version" v-if="modLoaderType && modLoaderVersion">
          <img src="@/assets/images/fabric.webp" fill="#fff" v-if="modLoaderType === 'Fabric'" />
          <img src="@/assets/images/quilt.svg" fill="#fff" v-if="modLoaderType === 'Quilt'" />
          <img
            src="@/assets/images/neoforged.png"
            fill="#fff"
            v-if="modLoaderType === 'Neoforge'" />
          <img
            src="@/assets/images/Anvil_JE3_BE3.webp"
            fill="#fff"
            v-if="modLoaderType === 'Forge'" />
          {{ modLoaderType }} {{ modLoaderVersion }}
        </div>
      </div>
      <i class="button scroll" @click="$emit('showLogs')"></i>
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
          @click="$emit('game-button-click')"
          v-if="!buttonLoading">
          <i
            :class="props.gameButtonType"
            style="font-family: fa-pro; font-style: normal; margin-right: 5px; font-weight: 100"></i
          >{{ gameButtonText }}
        </button>
        <button
          class="game-button loading"
          :class="`${gameButtonType}-game-button`"
          v-if="buttonLoading">
          <div class="a"></div>
          <div class="b"></div>
          <div class="c"></div>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import $ from "jquery";
import gsap from "gsap";
import { listen } from "@tauri-apps/api/event";

const i18n = useI18n();

const props = defineProps<{
  minecraftVersion: String;
  modLoaderVersion: String | undefined;
  modLoaderType: "Fabric" | "Forge" | "Quilt" | "Neoforge" | undefined;
  instanceName: String;
  installed: Boolean;
  gameButtonType: "install" | "launch" | "error";
  buttonLoading: boolean;
  errorType?: "install" | "launch";
}>();

let computedInstanceName = computed(() => {
  if (props.instanceName == "Latest Release") {
    return i18n.t("game.latestRelease");
  }
  if (props.instanceName == "Latest Snapshot") {
    return i18n.t("game.latestSnapshot");
  }
  return props.instanceName;
});

let banner = "";
let gameButtonText = computed(() => {
  switch (props.gameButtonType) {
    case "install":
      return i18n.t("game.install");
    case "launch":
      return i18n.t("game.launch");
    default:
      return "";
  }
});

function star() {
  let star = $("#star");
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
.instance-info {
  width: 100%;
  height: 240px;
  height: 50%;
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
}

.minecraft-version,
.mod-loader-version,
.launch-progress {
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
  display: inline-block;
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

  > div {
    width: 5px;
    height: 5px;
    background: #fff;
    opacity: 0.5;
    display: inline-block;
    margin: 0 1px;
    margin-bottom: 2px;
  }

  .a {
  }
}
</style>
