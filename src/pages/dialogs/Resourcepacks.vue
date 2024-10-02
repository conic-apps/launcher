<template>
  <dialog-vue :visible="props.show" :width="860" :height="520">
    <div class="worlds">
      <div style="width: 100%; height: 100%">
        <div class="title">
          <div style="display: flex; align-items: center">
            <div class="icon">
              <i class="map"></i>
            </div>
            <div>
              <h4>资源包</h4>
              <p>你可以在右侧列表选中资源包，然后点击添加按钮，将已下载的资源包添加到当前实例</p>
            </div>
          </div>
          <div class="button" style="position: absolute; right: 0" @click="$emit('close')">
            <i></i>
          </div>
        </div>

        <div class="content">
          <div class="row1">
            <div>
              <list-item
                v-for="(resourcepack, index) in datas"
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
                    text-color="#ffffffd0"></tag>
                </template>
                {{ resourcepack.metadata.description }}
              </list-item>
            </div>
          </div>
          <div class="row2">
            <p>TODO</p>
          </div>
        </div>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";

const props = defineProps<{
  show: boolean;
  instanceName: string;
  datas: Resourcepack[];
}>();

export type Resourcepack = {
  icon: string;
  metadata: {
    description: string;
    pack_format: number;
    other: any;
  };
  name: string;
  type: "unknown" | "texture" | "data";
};
</script>

<style lang="less" scoped>
.worlds {
  width: 100%;
  height: 100%;
  overflow: hidden;

  .title {
    border-bottom: 1px solid #ffffff18;
    width: 100%;
    height: 80px;
    display: flex;
    justify-content: space-between;
    position: relative;

    .icon {
      width: 80px;
      height: 80px;
      display: flex;
      align-items: center;
      justify-content: center;

      .back {
        border-radius: 1000px;
        width: 40px;
        height: 40px;
        border: 1px solid rgba(255, 255, 255, 0.38);
      }

      .back:active {
        opacity: 0.7;
      }

      .back::before {
        font-size: 20px;
      }
    }

    i {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    i::before {
      font-size: 36px;
      font-style: normal;
      font-family: "fa-pro";
    }

    h4 {
      font-size: 22px;
      font-weight: normal;
    }

    p {
      font-size: 14px;
      margin-top: 4px;
      opacity: 0.7;
      font-weight: normal;
    }

    .button {
      width: 20px;
      height: 20px;
      border-radius: 50%;
      margin-left: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: transform 100ms;
      background: #ffffff40;

      i {
        transition: all 100ms ease;
      }

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
    }

    .button:hover {
      i::before {
        opacity: 1;
      }
    }

    .button:active {
      i {
        opacity: 0.7;
      }
    }
  }

  .content {
    display: flex;
    height: calc(100% - 80px);
    padding-top: 16px;

    .row1 {
      width: 50%;
      height: 100%;
      padding: 0 12px;
      overflow: auto;

      > div {
        border-radius: 8px;
        overflow: hidden;
      }

      .list-item {
        width: 100%;
      }
    }

    .row2 {
      width: 50%;
      height: 100%;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 10px;
      display: flex;
      align-items: center;
      justify-content: center;

      p {
        font-style: italic;
        opacity: 0.6;
      }
    }
  }
}
</style>
<!-- <template> -->
<!--   <dialog-vue :visible="show" width="460" height="480"> -->
<!--     <div style="position: relative; margin: 12px 14px; width: calc(100% - 28px)"> -->
<!--       <div -->
<!--         style=" -->
<!--           display: flex; -->
<!--           justify-content: space-between; -->
<!--           border-bottom: 2px solid rgba(var(--theme-color), 0.6); -->
<!--           margin-bottom: 10px; -->
<!--         "> -->
<!--         <div class="info"> -->
<!--           <div class="icon"></div> -->
<!--           <div class="text"> -->
<!--             <h4 class="name"> -->
<!--               <span>{{ instanceName }}</span -->
<!--               >中的资源包 -->
<!--             </h4> -->
<!--             <p>共安装有 {{ resourcepacks.length }} 个资源包</p> -->
<!--           </div> -->
<!--         </div> -->
<!--         <div class="buttons"> -->
<!--           <dialog-button icon="close" @click="$emit('close')"></dialog-button> -->
<!--         </div> -->
<!--       </div> -->
<!--       <search-bar -->
<!--         style=" -->
<!--           margin-bottom: 8px; -->
<!--           position: sticky; -->
<!--           top: 0; -->
<!--           right: 0; -->
<!--           bottom: 0; -->
<!--           left: 0; -->
<!--           z-index: 1000; -->
<!--           background: #fff; -->
<!--           border: 1px solid #00000028; -->
<!--           box-shadow: 0 0 10px #00000012; -->
<!--         "></search-bar> -->
<!--       <TransitionGroup> -->
<!--         <list-item -->
<!--           v-for="(resourcepack, index) in resourcepacks" -->
<!--           :key="index" -->
<!--           :title="resourcepack.name" -->
<!--           :logo="resourcepack.icon" -->
<!--           :click-able="false" -->
<!--           :buttons="['circle-info', 'folders', 'trash-can']"> -->
<!--           <template #subtitle v-if="resourcepack.type == 'texture'"> -->
<!--             <tag -->
<!--               text="纹理包" -->
<!--               :color="['74', '194', '107']" -->
<!--               style="transform: scale(0.9)" -->
<!--               :border="true" -->
<!--               text-color="#1f883d"></tag> -->
<!--           </template> -->
<!--           <template #subtitle v-else-if="resourcepack.type == 'data'"> -->
<!--             <tag -->
<!--               text="数据包" -->
<!--               :color="['200', '200', '0']" -->
<!--               style="transform: scale(0.9)" -->
<!--               :border="true" -->
<!--               text-color="#9a6700"> -->
<!--             </tag> -->
<!--           </template> -->
<!--           <template #subtitle v-else> -->
<!--             <tag -->
<!--               text="未知" -->
<!--               :color="['200', '200', '200']" -->
<!--               :border="true" -->
<!--               text-color="#000000d0"></tag> -->
<!--           </template> -->
<!--           {{ resourcepack.description }} -->
<!--         </list-item> -->
<!--       </TransitionGroup> -->
<!--     </div> -->
<!--   </dialog-vue> -->
<!-- </template> -->
<!---->
<!-- <script setup lang="ts"> -->
<!-- import { reactive, ref } from "vue"; -->
<!-- import DialogVue from "@/components/Dialog.vue"; -->
<!-- import ListItem from "@/components/ListItem.vue"; -->
<!-- import Tag from "@/components/Tag.vue"; -->
<!-- import SearchBar from "@/components/SearchBar.vue"; -->
<!-- import DialogButton from "@/components/DialogButton.vue"; -->
<!---->
<!-- const props = withDefaults( -->
<!--   defineProps<{ -->
<!--     show: boolean; -->
<!--     resourcepacks?: any; -->
<!--     instanceName: string; -->
<!--     datas: any[]; -->
<!--   }>(), -->
<!--   { -->
<!--     resourcepacks: () => { -->
<!--       return [ -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--           type: "data", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--           type: "texture", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--           type: "texture", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--           type: "texture", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--           type: "lib", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--           type: "broken", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--         }, -->
<!--         { -->
<!--           name: "a", -->
<!--           description: "bbbbbbb", -->
<!--           icon: "/test.webp", -->
<!--         }, -->
<!--       ]; -->
<!--     }, -->
<!--   }, -->
<!-- ); -->
<!-- </script> -->
<!---->
<!-- <style lang="less" scoped> -->
<!-- .info { -->
<!--   display: flex; -->
<!--   align-items: center; -->
<!--   padding-bottom: 6px; -->
<!-- } -->
<!---->
<!-- .info .icon { -->
<!--   width: 40px; -->
<!--   height: 40px; -->
<!--   background: url(@/assets/images/Ancient_Debris.webp); -->
<!--   background-position: center; -->
<!--   background-size: cover; -->
<!-- } -->
<!---->
<!-- .info .text { -->
<!--   margin-left: 6px; -->
<!-- } -->
<!---->
<!-- .info h4 { -->
<!--   font-weight: 400; -->
<!--   font-size: 18px; -->
<!--   margin-bottom: 2px; -->
<!-- } -->
<!---->
<!-- .info span { -->
<!--   color: rgb(var(--theme-color)); -->
<!-- } -->
<!---->
<!-- .info p { -->
<!--   color: #000000a0; -->
<!--   font-size: 13px; -->
<!-- } -->
<!-- </style> -->
