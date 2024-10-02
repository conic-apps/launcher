<template>
  <dialog-vue :visible="show" :width="460" :height="480">
    <div class="main">
      <search-bar style="margin-bottom: 10px"> </search-bar>
      <div
        style="
          border-radius: 10px;
          overflow: hidden;
          border: 1px solid rgba(0, 0, 0, 0.16);
          box-shadow: rgba(0, 0, 0, 0.16) 0 0 10px;
        ">
        <list-item
          title="不选择 Forge"
          :click-able="true"
          @click="$emit('select', '')"
          :key="0"></list-item>
        <list-item
          v-for="(version, index) in versions"
          :key="index + 1"
          :title="version.version"
          logo="1"
          :click-able="true"
          @click="$emit('select', version.version)"
          :buttons="['circle-info', 'floppy-disk']"
          :description="`适用于 Minecraft ${props.minecraft}`">
          <template #icon>
            <img
              style="width: 100%; height: 100%"
              src="@/assets/images/Anvil_JE3_BE3.webp"
              alt="" />
          </template>
          <template #subtitle> </template>
        </list-item>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import SearchBar from "@/components/SearchBar.vue";
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";

const props = defineProps<{
  show?: boolean;
  minecraft: string;
}>();
const emit = defineEmits(["no-version", "loaded", "select"]);

let versions = ref<Array<any>>([]);

watch(props, (newValue) => {
  if (newValue.minecraft != "") {
    invoke("get_forge_version_list", {
      mcversion: newValue.minecraft,
    }).then((res: any) => {
      if (res != null && res.length > 0) {
        versions.value = res;
        emit("loaded");
      } else {
        emit("no-version");
      }
    });
  }
});
</script>

<style lang="less" scoped>
.main {
  height: 100%;
  width: 100%;
  margin: 12px 14px;
  width: calc(100% - 28px);
}
</style>
