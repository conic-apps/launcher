<template>
  <div>
    <ul class="gamelist" id="gamelist">
      <div style="display: flex; align-items: center; margin-bottom: 8px;">
        <search-bar style="width: auto;"></search-bar>
        <i style="flex-shrink: 0;margin-left: 6px; margin-right: 4px;" @click="$emit('jump', 'allInstances')"
          class="grid-2 new"></i>
        <i style="flex-shrink: 0;" @click="$emit('jump', 'newInstance')" class="plus new"></i>
      </div>
      <TransitionGroup name="list-item">
        <li v-for="(instance, index) in instances" :key="index">
          <img src="@/assets/images/Grass_Block.webp">{{ instance.name }}
        </li>
      </TransitionGroup>
      <p v-if="instances.length == 0">此视图筛选条件无匹配结果</p>
    </ul>
    <div class="content">
      <div class="version" :style="banner">
        <div>
          <div class="minecraft-version"><img src="@/assets/images/minecraft.webp">Minecraft {{ minecraftVersion }}
          </div>
        </div>
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <p>{{ instanceName }}</p>
          <div style="display: flex; align-items: center;">
            <i class="button gear"></i>
            <i class="button circle-info"></i>
            <i class="button star"></i>
            <div class="start-game" v-if="installed" @click="launchGame"><i class="play"
                style="font-family: 'fa-pro'; font-style: normal; margin-right: 5px; font-weight: 100;"></i>开始游戏
            </div>
            <div class="install-game" v-else @click="installGame"><i class="download"
                style="font-family: 'fa-pro'; font-style: normal; margin-right: 5px; font-weight: 400;"></i>安装
            </div>
          </div>
        </div>
      </div>
      <div class="assets">
        <div>
          <card-link icon="map" title="地图存档" description="正在加载..." margin="0,0,10,0"
            @click="show.worlds = true"></card-link>
          <card-link icon="puzzle-piece" title="模组" description="正在加载..." margin="0,0,10,0"
            @click="show.mods = true"></card-link>
          <card-link icon="puzzle-piece" title="截图" description="正在加载..." margin="0,0,0,0"></card-link>
        </div>
        <div>
          <card-link icon="palette" title="资源包" description="正在加载..." margin="0,0,10,0"
            @click="show.resourcepacks = true"></card-link>
          <card-link icon="lightbulb-on" title="光影包" description="正在加载..." margin="0,0,10,0"
            @click="show.shaderpacks = true"></card-link>
          <card-link icon="puzzle-piece" title="日志" description="正在加载..." margin="0,0,0,0"></card-link>
        </div>
      </div>
    </div>
    <worlds :show="show.worlds" instance-name="未命名配置" @close="show.worlds = false"></worlds>
    <mods :show="show.mods" instance-name="未命名配置" @close="show.mods = false"></mods>
    <resourcepacks :show="show.resourcepacks" instance-name="未命名配置" @close="show.resourcepacks = false"></resourcepacks>
    <shaderpacks :show="show.shaderpacks" instance-name="未命名配置" @close="show.shaderpacks = false"></shaderpacks>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import cardLink from '@/components/CardLink.vue'
import SearchBar from '@/components/SearchBar.vue'
import Worlds from './dialogs/Worlds.vue'
import Mods from './dialogs/Mods.vue'
import Resourcepacks from './dialogs/Resourcepacks.vue'
import Shaderpacks from './dialogs/Shaderpacks.vue'

let show = reactive({
  worlds: false,
  mods: false,
  resourcepacks: false,
  shaderpacks: false
})
let banner = ref("background-image: linear-gradient(0deg, rgb(0 0 0 / 83%), rgb(0 0 0 / 0%)), url(./src/assets/images/banners/1.18.webp);")
let instanceName = "Minecraft 1.18.2 with fabric"
let minecraftVersion = ref("1.18.2")
let installed = ref(false)
let instances: any = ref([])
instances.value = [
  {
    name: "未命名配置",
    metadata: JSON.parse('{"name":"未命名配置","runtime":{"minecraft":"1.19.4","forge":"","liteloader":"","fabricLoader":"","optifine":"","quilt":""},"author":"","version":"1.19.4","tags":[],"icon":"","broken":false}'),
    installed: false,
    banner: '@/assets/images/banners/1.13.webp'
  },
  {
    name: "未命名配置",
    metadata: JSON.parse('{"name":"未命名配置","runtime":{"minecraft":"1.19.4","forge":"","liteloader":"","fabricLoader":"","optifine":"","quilt":""},"author":"","version":"1.19.4","tags":[],"icon":"","broken":false}'),
    installed: false,
    banner: '@/assets/images/banners/1.13.webp'
  },
  {
    name: "未命名配置",
    metadata: JSON.parse('{"name":"未命名配置","runtime":{"minecraft":"1.19.4","forge":"","liteloader":"","fabricLoader":"","optifine":"","quilt":""},"author":"","version":"1.19.4","tags":[],"icon":"","broken":false}'),
    installed: false,
    banner: '@/assets/images/banners/1.13.webp'
  },
  {
    name: "未命名配置",
    metadata: JSON.parse('{"name":"未命名配置","runtime":{"minecraft":"1.19.4","forge":"","liteloader":"","fabricLoader":"","optifine":"","quilt":""},"author":"","version":"1.19.4","tags":[],"icon":"","broken":false}'),
    installed: false,
    banner: '@/assets/images/banners/1.13.webp'
  }
]

function launchGame() {

}

function installGame() {

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

.minecraft-version {
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
  font-family: 'fa-pro';
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
.install-game {
  margin-left: 8px;
  border-radius: 8px;
  font-size: calc(15px - var(--font-size-error));
  padding: 8px 20px;
  color: #fff;
  cursor: pointer;
  transition: all 100ms ease;
  display: inline-block;
  overflow: hidden;
  font-family: 'Harmony-light';
}

.start-game {
  background-image: linear-gradient(248deg, #18b14e, #4fc82f);
}

.install-game {
  width: 116.25px;
  height: 36px;
  background-image: linear-gradient(248deg, #235dce, #399bed);
  letter-spacing: 1px;
  text-align: center;
}

ul.gamelist {
  width: 280px;
  flex-shrink: 0;
  margin-left: -60px;
  padding: 30px 0px 0 82px;
  // background-color: #ffffff8b;
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
  font-family: 'fa-pro';
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
}</style>
