<template>
  <div>
    <!-- <ul :class="canChangeInstance ? 'gamelist' : 'disabled gamelist'" style="transition: opacity .3s ease;" id="gamelist">
      <div style="display: flex; align-items: center; margin-bottom: 8px;">
        <search-bar style="width: auto;"></search-bar>
        <i style="flex-shrink: 0;margin-left: 6px; margin-right: 4px;" @click="$emit('jump', 'allInstances')"
          class="grid-2 new"></i>
        <i style="flex-shrink: 0;" @click="$emit('jump', 'newInstance')" class="plus new"></i>
      </div>
      <TransitionGroup name="list-item" tag="ul">
        <li v-for="(instance, index) in instances" :key="index"
          :class="activeInstance.config?.name == instance.config.name ? 'active' : ''" @click="activeInstance = instance">
          <img src="@/assets/images/Grass_Block.webp">{{ instance.config.name }}
        </li>
      </TransitionGroup>
      <p v-if="instances.length == 0">启动器损坏</p>
    </ul> -->
    <div class="content">
      <div class="version" :style="banner">
        <div style="
            display: flex;
            align-items: start;
            justify-content: space-between;
          ">
          <div class="minecraft-version">
            <img src="@/assets/images/minecraft.webp" />Minecraft
            {{ minecraftVersion }}
          </div>
          <div class="progress-info" :style="!canChangeInstance ? `` : `opacity: 0;`">
            <p>{{ progressMessage }}</p>
            <progress-bar :loading="progressLoading" :value="progressValue" width="180"></progress-bar>
          </div>
        </div>
        <div style="
            display: flex;
            justify-content: space-between;
            align-items: center;
          ">
          <p>{{ activeInstance.config?.name }}</p>
          <div style="display: flex; align-items: center">
            <i class="button gear"></i>
            <i class="button circle-info"></i>
            <i class="button star"></i>
            <div class="installing" v-if="!canChangeInstance">
              <div class="button-loading"></div>
            </div>
            <div class="start-game" v-else-if="installed" @click="launchGame">
              <i class="play" style="
                  font-family: &quot;fa-pro&quot;;
                  font-style: normal;
                  margin-right: 5px;
                  font-weight: 100;
                "></i>开始游戏
            </div>
            <div class="install-game" v-else @click="installGame">
              <i class="download" style="
                  font-family: &quot;fa-pro&quot;;
                  font-style: normal;
                  margin-right: 5px;
                  font-weight: 400;
                "></i>安装
            </div>
          </div>
        </div>
      </div>
      <div class="assets">
        <div>
          <card-link icon="map" title="地图存档" :class="savesIsLoading ? 'disabled' : ''" margin="0,0,10,0"
            :description="savesManagerDesc" @click="show.worlds = true"></card-link>
          <card-link icon="puzzle-piece" title="模组" :class="modIsLoading ? 'disabled' : ''"
            :description="modManagerDesc" margin="0,0,10,0" @click="show.mods = true"></card-link>
          <card-link icon="puzzle-piece" title="截图" margin="0,0,0,0"></card-link>
        </div>
        <div>
          <card-link icon="palette" title="资源包" :class="resourcepacksIsLoading ? 'disabled' : ''"
            :description="resourcepacksManagerDesc" margin="0,0,10,0" @click="show.resourcepacks = true"></card-link>
          <card-link icon="lightbulb-on" title="光影包" :class="shaderpackIsLoading ? 'disabled' : ''"
            :description="shaderpacksManagerDesc" margin="0,0,10,0" @click="show.shaderpacks = true"></card-link>
          <card-link icon="puzzle-piece" title="日志" margin="0,0,0,0"></card-link>
        </div>
      </div>
    </div>
    <worlds :show="show.worlds" :datas="saves" :instance-name="activeInstance.config?.name"
      @close="show.worlds = false">
    </worlds>
    <mods :show="show.mods" :datas="mods" :instance-name="activeInstance.config?.name" @close="show.mods = false">
    </mods>
    <resourcepacks :show="show.resourcepacks" :datas="resourcepacks" :instance-name="activeInstance.config?.name"
      @close="show.resourcepacks = false"></resourcepacks>
    <shaderpacks :show="show.shaderpacks" :datas="shaderpacks" :instance-name="activeInstance.config?.name"
      @close="show.shaderpacks = false">
    </shaderpacks>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import cardLink from "@/components/CardButton.vue";
import SearchBar from "@/components/SearchBar.vue";
import Worlds from "./dialogs/Worlds.vue";
import Mods from "./dialogs/Mods.vue";
import Resourcepacks from "./dialogs/Resourcepacks.vue";
import Shaderpacks from "./dialogs/Shaderpacks.vue";
import { event } from "@tauri-apps/api";
import { invoke } from "@tauri-apps/api/core";
import ProgressBar from "@/components/ProgressBar.vue";
let show = reactive({
  worlds: false,
  mods: false,
  resourcepacks: false,
  shaderpacks: false,
});
let banner = ref(
  "background-image: linear-gradient(0deg, rgb(0 0 0 / 83%), rgb(0 0 0 / 0%)), url(./src/assets/images/banners/1.18.webp);",
);
let minecraftVersion = ref("1.18.2");
let installed = ref(false);
let instances = ref<any>([]);
let activeInstance = ref<any>({});
let canChangeInstance = ref(true); // 当安装或启动游戏时,禁止更改视图以防止数据竞争

let mods = ref<any>([]);
let saves = ref<any>([]);
let resourcepacks = ref<any>([]);
let shaderpacks = ref<any>([]);

watch(activeInstance, async (newValue) => {
  await invoke("set_active_instance", {
    instanceName: newValue.config.name,
  });
  installed.value = newValue.installed;
  updateData();
  // banner.value = `background-image: linear-gradient(0deg, rgb(0 0 0 / 83%), rgb(0 0 0 / 0%)), url(./src/assets/images/banners/${val}.webp)`
});

let resourcepacksIsLoading = ref(true);
let modIsLoading = ref(true);
let shaderpackIsLoading = ref(true);
let savesIsLoading = ref(true);

let modManagerDesc = computed(() => {
  // todo: 不过滤无法识别的模组，因为这会导致用户不能禁用某些废物的不规范模组
  if (modIsLoading.value) {
    return "正在加载...";
  } else {
    return `已安装 ${mods.value.length} 个模组`;
  }
});
let resourcepacksManagerDesc = computed(() => {
  if (resourcepacksIsLoading.value) {
    return "正在加载...";
  } else {
    return `已安装 ${resourcepacks.value.length} 个资源包`;
  }
});
let shaderpacksManagerDesc = computed(() => {
  if (shaderpackIsLoading.value) {
    return "正在加载...";
  } else {
    return `已安装 ${shaderpacks.value.length} 个光影包`;
  }
});
let savesManagerDesc = computed(() => {
  if (savesIsLoading.value) {
    return "正在加载...";
  } else {
    return `共有 ${saves.value.length} 个存档`;
  }
});

invoke("watch_instances_folder");

function update() {
  invoke("scan_instances_folder").then((res: any) => {
    instances.value = res;
  });
}

update();

function updateData() {
  modIsLoading.value = true;
  resourcepacksIsLoading.value = true;
  shaderpackIsLoading.value = true;
  savesIsLoading.value = true;
  invoke("scan_mod_folder").then((res: any) => {
    mods.value = res.sort((a: any, b: any) => a.name.localeCompare(b.name));
    modIsLoading.value = false;
  });
  invoke("scan_saves_folder").then((res: any) => {
    saves.value = res;
    savesIsLoading.value = false;
  });
}

let progressMessage = ref("");
let progressValue = ref("");
let progressLoading = ref(false);

interface InstallProgress {
  completed: number;
  total: number;
  step: number;
}

event.listen("instances_changed", (data: any) => {
  update();
  updateData();
});
event.listen("install_progress", (data: any) => {
  progressMessage.value = JSON.stringify(data.payload);
  let progress = data.payload as InstallProgress;
  switch (progress.step) {
    case 1:
      canChangeInstance.value = false;
      progressMessage.value = "正在准备安装";
      progressValue.value = progress.completed.toString();
      progressLoading.value = true;
      break;
    case 2:
      canChangeInstance.value = false;
      progressMessage.value = "正在准备安装";
      progressValue.value = progress.completed.toString();
      progressLoading.value = true;
      break;
    case 3:
      canChangeInstance.value = false;
      progressMessage.value = `正在下载: ${((progress.completed / progress.total) * 100).toFixed(2)}%`;
      progressValue.value = (
        (progress.completed / progress.total) *
        100
      ).toString();
      progressLoading.value = false;
  }
  if (progress.completed == progress.total && progress.step == 3) {
    progressMessage.value = "安装完成";
    setTimeout(() => {
      canChangeInstance.value = true;
      update();
      installed.value = true;
    }, 1000);
  }
});

function launchGame() {
  canChangeInstance.value = false;
}
function installGame() {
  canChangeInstance.value = false;
  invoke("install_command", {});
}
</script>

<style lang="less" scoped>
.content {
  padding: 20px 34px 0 18px;
}

.version {
  height: 240px;
  width: 100%;
  padding: 16px 22px 16px 24px;
  border-radius: 8px;
  background-size: cover;
  background-position: center;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  box-shadow: 0 0 0 1px #ffffff29;
  color: #fff;
}

.minecraft-version,
.progress-info {
  display: flex;
  align-items: center;
  justify-items: center;
  border: 1px solid #ffffff2f;
  background-color: #00000042;
  backdrop-filter: blur(3px);
  -webkit-backdrop-filter: blur(3px);
  padding: 5px 14px;
  width: fit-content;
  border-radius: 6px;
  font-size: calc(15px - var(--font-size-error));
  transition: opacity 0.3s ease;
}

.minecraft-version img {
  width: 22px;
  height: 22px;
  margin-right: 4px;
}

.version>div:last-child p {
  font-size: calc(20px - var(--font-size-error));
}

i.button {
  font-family: "fa-pro";
  font-style: normal;
  font-size: calc(18px - var(--font-size-error));
  border-radius: 100px;
  margin: 0 2px;
  width: 44px;
  height: 44px;
  display: inline-flex;
  transition: all 100ms ease-in-out;
}

i.button:hover {
  background: #ffffff23;
  transform: scale(1.04);
}

i.button:active {
  background: #ffffff15;
  transform: scale(0.96);
}

i.button::before {
  margin: auto;
}

.start-game,
.install-game,
.installing {
  margin-left: 8px;
  border-radius: 8px;
  font-size: calc(15px - var(--font-size-error));
  padding: 8px 20px;
  color: #fff;
  cursor: pointer;
  transition: all 100ms ease;
  display: inline-block;
  overflow: hidden;
}

.start-game {
  background-image: linear-gradient(248deg, #18b14e, #4fc82f);
}

.install-game,
.installing {
  width: 116.25px;
  height: 36px;
  background-image: linear-gradient(248deg, #235dce, #399bed);
  letter-spacing: 1px;
  text-align: center;
}

.installing {
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: default;
}

ul.gamelist {
  width: 280px;
  flex-shrink: 0;
  margin-left: -60px;
  padding: 30px 0px 0 82px;
  // background-color: #ffffff8b;
}

ul.gamelist>ul {
  overflow: overlay;
  height: calc(100% - 54px);
}

ul.gamelist img {
  width: 18px;
  height: 18px;
  margin-right: 4px;
}

ul.gamelist li {
  margin-bottom: 4px;
  display: flex;
  align-items: center;
  padding: 8px 10px;
  border-radius: var(--border-radius-small);
  transition: all 50ms ease;
  font-size: calc(15px - var(--font-size-error));
}

ul.gamelist li:hover {
  background: #00000012;
}

ul.gamelist li:active {
  background-color: #00000015;
}

ul.gamelist li.active {
  background: rgba(var(--theme-color), 0.09);
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.1);
  pointer-events: none;
  color: rgba(var(--theme-color), 0.9);
}

ul.gamelist li::before {
  background-color: #007bff00;
  content: "";
  height: 14px;
  width: 4px;
  position: relative;
  left: -7px;
  border-radius: 5px;
  transition: all 0.15s ease;
}

ul.gamelist li.active::before {
  background-color: rgba(var(--theme-color), 1);
  content: "";
  height: 18px;
}

.assets {
  display: flex;
  margin-top: 14px;
}

.assets>div {
  display: flex;
  flex-direction: column;
  width: 100%;
}

.assets>div:first-child {
  margin-right: 5px;
}

.assets>div:last-child {
  margin-left: 5px;
}

.new {
  font-family: "fa-pro";
  width: 8px;
  height: 8px;
  font-size: calc(16px - var(--font-size-error));
  font-style: normal;
  margin-left: 2px;
  display: flex;
  justify-content: center;
  align-items: center;
  border-radius: 10000px;
  padding: 10px;
  transition: all 200ms ease;
}

.new:hover {
  color: rgba(var(--theme-color), 1);
}

.new:active {
  transform: scale(0.9);
}

.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.progress-info {
  width: 220px; // remove
  flex-direction: column;
  align-items: start;
  justify-self: center;
  padding: 12px 18px;
}

.progress-info p {
  font-size: calc(15px - var(--font-size-error));
  padding-bottom: 4px;
}

// .progress progress::-webkit-progress-value {

// }
.button-loading,
.button-loading::before,
.button-loading::after {
  width: 5px;
  height: 5px;
  background-color: rgba(255, 255, 255, 0.2);
  animation: 0.8s button-loading infinite linear alternate;
}

.button-loading {
  position: relative;
  animation-delay: 1.2s;
}

.button-loading::before,
.button-loading::after {
  content: "";
  display: inline-block;
  position: absolute;
  top: 0;
}

.button-loading::after {
  left: -6.5px;
  animation-delay: 1s;
}

.button-loading::before {
  left: 6.5px;
  animation-delay: 1.4s;
}

@keyframes button-loading {
  0% {
    background-color: rgba(255, 255, 255, 0.2);
  }

  70% {
    background-color: rgba(255, 255, 255, 0.2);
  }

  100% {
    background-color: rgba(255, 255, 255, 0.75);
  }
}
</style>
