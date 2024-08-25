<template>
  <div class="assets">
    <div class="first-row">
      <card-button
        icon="map"
        :title="$t('game.gameData.saves')"
        :class="savesIsLoading ? 'disabled' : ''"
        margin="0,0,10,0"
        :description="savesManagerDesc"
        @click="show.worlds = true"></card-button>
      <card-button
        icon="puzzle-piece"
        :title="$t('game.gameData.mods')"
        :class="modIsLoading ? 'disabled' : ''"
        :description="modManagerDesc"
        margin="0,0,10,0"
        @click="show.mods = true"></card-button>
      <card-button
        icon="puzzle-piece"
        :title="$t('game.gameData.resourcepacks')"
        margin="0,0,0,0"
        :description="$t('game.gameData.loading')"></card-button>
    </div>
    <div class="second-row">
      <card-button
        icon="palette"
        :title="$t('game.gameData.resourcepacks')"
        :class="resourcepacksIsLoading ? 'disabled' : ''"
        :description="resourcepacksManagerDesc"
        margin="0,0,10,0"
        @click="show.resourcepacks = true"></card-button>
      <card-button
        icon="lightbulb-on"
        :title="$t('game.gameData.shaderpacks')"
        :class="shaderpackIsLoading ? 'disabled' : ''"
        :description="shaderpacksManagerDesc"
        margin="0,0,10,0"
        @click="show.shaderpacks = true"></card-button>
      <card-button
        icon="puzzle-piece"
        :title="$t('game.gameData.schematics')"
        margin="0,0,0,0"
        description="正在加载"></card-button>
    </div>
    <worlds
      :show="show.worlds"
      :datas="saves"
      :instance-name="instance.config.name"
      @close="show.worlds = false">
    </worlds>
    <mods
      :show="show.mods"
      :datas="mods"
      :instance-name="instance.config.name"
      @close="show.mods = false"></mods>
    <resourcepacks
      :show="show.resourcepacks"
      :datas="resourcepacks"
      :instance-name="instance.config.name"
      @close="show.resourcepacks = false"></resourcepacks>
    <shaderpacks
      :show="show.shaderpacks"
      :datas="shaderpacks"
      :instance-name="instance.config.name"
      @close="show.shaderpacks = false">
    </shaderpacks>
  </div>
</template>

<style lang="less" scoped>
.assets {
  display: flex;
  margin-top: 14px;
}

.assets > div {
  display: flex;
  flex-direction: column;
  width: 100%;
}

.assets > div.first-row {
  margin-right: 5px;
}

.assets > div.second-row {
  margin-left: 5px;
}
</style>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, reactive, ref, watch } from "vue";
import CardButton from "./CardButton.vue";
import Worlds from "@/pages/dialogs/Worlds.vue";
import { Save } from "@/pages/dialogs/Worlds.vue";
import Mods from "@/pages/dialogs/Mods.vue";
import Resourcepacks from "@/pages/dialogs/Resourcepacks.vue";
import Shaderpacks from "@/pages/dialogs/Shaderpacks.vue";
import { Instance } from "./Instances.vue";
import { useI18n } from "vue-i18n";
const i18n = useI18n();

const props = defineProps<{
  instance: Instance;
}>();
let show = reactive({
  worlds: false,
  mods: false,
  resourcepacks: false,
  shaderpacks: false,
});

let mods = ref<any>([]);
let saves = ref<Save[]>([]);
let resourcepacks = ref<any>([]);
let shaderpacks = ref<any>([]);

let resourcepacksIsLoading = ref(true);
let modIsLoading = ref(true);
let shaderpackIsLoading = ref(true);
let savesIsLoading = ref(true);
let modManagerDesc = computed(() => {
  // todo: 不过滤无法识别的模组，因为这会导致用户不能禁用某些废物的不规范模组
  if (modIsLoading.value) {
    return i18n.t("game.gameData.loading");
  } else {
    return i18n.t("game.gameData.modsCount", { count: mods.value.length });
  }
});
let resourcepacksManagerDesc = computed(() => {
  if (resourcepacksIsLoading.value) {
    return i18n.t("game.gameData.loading");
  } else {
    return i18n.t("game.gameData.resourcepacksCount", { count: resourcepacks.value.length });
  }
});
let shaderpacksManagerDesc = computed(() => {
  if (shaderpackIsLoading.value) {
    return i18n.t("game.gameData.loading");
  } else {
    return i18n.t("game.gameData.shaderpacksCount", { count: shaderpacks.value.length });
  }
});
let savesManagerDesc = computed(() => {
  if (savesIsLoading.value) {
    return i18n.t("game.gameData.loading");
  } else {
    return i18n.t("game.gameData.savesCount", { count: saves.value.length });
  }
});

watch(props, (newValue) => {
  updateData();
});

// watch(show, (newValue) => {
//   if (newValue.mods) {
//   }
// });

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
  invoke("scan_resourcepack_folder").then((res: any) => {
    resourcepacks.value = res;
    resourcepacksIsLoading.value = false;
  });
  invoke("scan_shaderpack_folder").then((res: any) => {
    shaderpacks.value = res;
    shaderpackIsLoading.value = false;
  });
}
updateData();
</script>
