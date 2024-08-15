<template>
  <dialog-vue :visible="show" width="460" height="480">
    <div style="position: relative; margin: 12px 14px; width: calc(100% - 28px)">
      <div
        style="
          display: flex;
          justify-content: space-between;
          border-bottom: 2px solid rgba(var(--theme-color), 0.6);
          margin-bottom: 10px;
        ">
        <div class="info">
          <div class="icon"></div>
          <div class="text">
            <h4 class="name">
              <span>{{ instanceName }}</span
              >中的资源包
            </h4>
            <p>共安装有 {{ resourcepacks.length }} 个资源包</p>
          </div>
        </div>
        <div class="buttons">
          <dialog-button icon="close" @click="$emit('close')"></dialog-button>
        </div>
      </div>
      <search-bar
        style="
          margin-bottom: 8px;
          position: sticky;
          top: 0;
          right: 0;
          bottom: 0;
          left: 0;
          z-index: 1000;
          background: #fff;
          border: 1px solid #00000028;
          box-shadow: 0 0 10px #00000012;
        "></search-bar>
      <TransitionGroup>
        <list-item
          v-for="(resourcepack, index) in resourcepacks"
          :key="index"
          :title="resourcepack.name"
          :logo="resourcepack.icon"
          :click-able="false"
          :buttons="['circle-info', 'folders', 'trash-can']">
          <template #subtitle v-if="resourcepack.type == 'texture'">
            <tag
              text="纹理包"
              :color="['74', '194', '107']"
              style="transform: scale(0.9)"
              :border="true"
              text-color="#1f883d"></tag>
          </template>
          <template #subtitle v-else-if="resourcepack.type == 'data'">
            <tag
              text="数据包"
              :color="['200', '200', '0']"
              style="transform: scale(0.9)"
              :border="true"
              text-color="#9a6700">
            </tag>
          </template>
          <template #subtitle v-else>
            <tag
              text="未知"
              :color="['200', '200', '200']"
              :border="true"
              text-color="#000000d0"></tag>
          </template>
          {{ resourcepack.description }}
        </list-item>
      </TransitionGroup>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import DialogVue from "@/components/Dialog.vue";
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";
import SearchBar from "@/components/SearchBar.vue";
import DialogButton from "@/components/DialogButton.vue";

const props = withDefaults(
  defineProps<{
    show: boolean;
    resourcepacks?: any;
    instanceName: string;
    datas: any[];
  }>(),
  {
    resourcepacks: () => {
      return [
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
          type: "data",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
          type: "texture",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
          type: "texture",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
          type: "texture",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
          type: "lib",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
          type: "broken",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
        },
        {
          name: "a",
          description: "bbbbbbb",
          icon: "/test.webp",
        },
      ];
    },
  },
);
</script>

<style lang="less" scoped>
.info {
  display: flex;
  align-items: center;
  padding-bottom: 6px;
}

.info .icon {
  width: 40px;
  height: 40px;
  background: url(@/assets/images/Ancient_Debris.webp);
  background-position: center;
  background-size: cover;
}

.info .text {
  margin-left: 6px;
}

.info h4 {
  font-weight: 400;
  font-size: 18px;
  margin-bottom: 2px;
}

.info span {
  color: rgb(var(--theme-color));
}

.info p {
  color: #000000a0;
  font-size: 13px;
}
</style>
