<template>
  <div class="instance-info-main card" :style="banner">
    <div
      style="position: absolute; width: 100%; height: 100%; top: 0; left: 0; background-image: linear-gradient(rgba(0, 0, 0, 0.824), rgba(0,0,0,0), rgba(0, 0, 0, 0.777));">
    </div>
    <div class="line-a">
      <div class="minecraft-version">
        <img src="@/assets/images/minecraft-icon.svg" fill="#fff">Minecraft {{ minecraftVersion }}
      </div>
    </div>
    <div class="line-b">
      <div class="instance-name">
        {{ instanceName }}
      </div>
      <div class="controll-btns">
        <i class="button gear"></i>
        <i class="button circle-info"></i>
        <i class="button star"></i>
        <button class="game-button"><i class="play"
                style="font-family: 'fa-pro'; font-style: normal; margin-right: 5px; font-weight: 100;"></i>开始游戏</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  minecraftVersion: String,
  instanceName: String,
  installed: Boolean,
  gameButtonType: "installing" | "launching" | "install" | "launch" | "error",
}>()
let banner = ""
switch (props.gameButtonType) {
  case "installing":
    break;
  case "error":
    break;
  case "install":
    break;
  case 'launching':
    break;
  case 'launch':
    break;

}
</script>

<style lang="less" scoped>
.instance-info-main {
  width: 100%;
  height: 240px;
  padding: 20px 24px;
  display: flex;
  flex-direction: column;
  background-image: url('@/assets/images/banners/default.webp');
  background-position: center;
  background-repeat: none;
  background-size: cover;
  justify-content: space-between;
  border: none;
  position: relative;
  overflow: hidden;
  // border: 4px solid rgba(255, 255, 255, 0.174);
}

.minecraft-version {
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

.minecraft-version img {
  width: 22px;
  opacity: 0.7;
  margin-right: 6px;
}

.instance-name {
  font-size: 22px;
}

.line-a,
.line-b {
  z-index: 10;
}

.line-b {
  display: flex;
  justify-content: space-between;
}

.line-b .button {
  font-style: normal;
  font-family: 'fa-pro';
  font-weight: 400;
  font-size: 18px;
  border-radius: 100px;
  margin: 0 2px;
  width: 24px;
  height: 24px;
  margin-right: 16px;
  display: inline-flex;
  transition: all 50ms ease-in-out;
  align-items: center;
  justify-content: center;
  opacity: 0.8;
}

i.button::before {
  text-align: center;
}

i.button:hover {
  opacity: 1;
}

i.button:active {
  opacity: 0.86;
}

button.game-button {
  border: none;
  width: 120px;
  height: 36px;
  margin-left: 8px;
  border-radius: 8px;
  font-size: 14px;
  padding: 8px 10px;
  color: #fff;
  cursor: pointer;
  transition: all 100ms ease;
  display: inline-block;
  overflow: hidden;
  background-image: linear-gradient(248deg, #18b14e, #4fc82f);
}
</style>