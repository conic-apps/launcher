<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <dialog-vue :visible="props.show" :width="560" :height="468">
    <div class="create-instance">
      <p
        style="
          width: 100%;
          margin-top: -4px;
          margin-bottom: 16px;
          padding-bottom: 16px;
          border-bottom: var(--card-border);
        ">
        Create Instance
      </p>
      <div class="dialog-button" @click="close">
        <i></i>
      </div>
      <div class="content">
        <Transition mode="out-in" :name="transitionName">
          <div class="settings" v-if="currentComponent == 'settings'">
            <SettingGroup>
              <SettingItem title="Instance Name" icon="signature">
                <TextInputBox
                  width="260px"
                  :placeholder="defaultInstanceName"
                  v-model="instanceNameValue">
                </TextInputBox>
              </SettingItem>
            </SettingGroup>
            <SettingGroup>
              <SettingItem
                title="Minecraft Version"
                description="Choose a Minecraft version"
                icon="minecraft"
                :click-able="true"
                @click="
                  transitionName = 'slide-left';
                  currentComponent = 'choose-minecraft';
                ">
                <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{
                  minecraftVersion
                }}</span>
                <i class="chevron-right" style="margin-right: 10px"></i>
              </SettingItem>
            </SettingGroup>
            <SettingGroup>
              <SettingItem
                title="Mod Loader"
                :description="modLoaderListLoading ? 'Loading...' : 'Choose a mod loader'"
                icon="puzzle-piece"
                :disabled="!minecraftVersion || modLoaderListLoading">
                <icon-select
                  :options="['none', 'quilt', 'fabric', 'neoforged', 'forge']"
                  :icons="['fa-pro ban', 'quilt', 'fabric', 'neoforged', 'forge']"
                  :disabled="disabledModLoaderId"
                  v-model="modLoaderType"></icon-select>
              </SettingItem>
              <SettingItem
                title="Mod Loader Version"
                description="Choose mod loader version."
                icon="puzzle-piece"
                :disabled="!modLoaderType"
                :click-able="true"
                @click="
                  transitionName = 'slide-left';
                  if (modLoaderType === 1) {
                    currentComponent = 'choose-quilt';
                  }
                  if (modLoaderType === 2) {
                    currentComponent = 'choose-fabric';
                  }
                  if (modLoaderType === 3) {
                    currentComponent = 'choose-neoforged';
                  }
                  if (modLoaderType === 4) {
                    currentComponent = 'choose-forge';
                  }
                ">
                <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{
                  modLoaderVersion
                }}</span>
                <i class="chevron-right" style="margin-right: 10px"></i>
              </SettingItem>
            </SettingGroup>
            <div style="display: flex; padding: 0 8px; margin-top: -8px">
              <ButtonVue style="margin-right: 8px" @click="close">Cancel</ButtonVue>
              <ButtonVue @click="createInstance" :disabled="creating || !minecraftVersion">
                {{ creating ? "Creating..." : "Create Instance" }}
              </ButtonVue>
            </div>
          </div>
          <MinecraftChoose
            v-else-if="currentComponent == 'choose-minecraft'"
            @select="setMinecraft"></MinecraftChoose>
          <QuiltChoose
            v-else-if="currentComponent == 'choose-quilt'"
            :minecraft="minecraftVersion"
            :versions="quiltVersionList"
            @select="setQuilt">
          </QuiltChoose>
          <FabricChoose
            v-else-if="currentComponent == 'choose-fabric'"
            :minecraft="minecraftVersion"
            :versions="fabricVersionList"
            @select="setFabric"></FabricChoose>
          <ForgeChoose
            v-else-if="currentComponent == 'choose-forge'"
            :minecraft="minecraftVersion"
            :versions="forgeVersionList"
            @select="setForge">
          </ForgeChoose>
          <NeoforgedChoose
            v-else-if="currentComponent == 'choose-neoforged'"
            :minecraft="minecraftVersion"
            :versions="neoforgedVersionList"
            @select="setNeoforged">
          </NeoforgedChoose>
        </Transition>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import TextInputBox from "@/components/TextInputBox.vue";
import { computed, ref, watch, watchEffect } from "vue";
import ButtonVue from "@/components/Button.vue";
import IconSelect from "@/components/IconSelect.vue";
import MinecraftChoose from "./create/MinecraftChoose.vue";
import QuiltChoose from "./create/QuiltChoose.vue";
import FabricChoose from "./create/FabricChoose.vue";
import ForgeChoose from "./create/ForgeChoose.vue";
import NeoforgedChoose from "./create/NeoforgedChoose.vue";
import {
  FabricLoaderArtifact,
  ForgeVersionItem,
  getFabricVersionList,
  getForgeVersionList,
  getNeoforgedVersionList,
  getQuiltVersionList,
  QuiltVersion,
} from "@conic/install";
import { createInstance as conicCreateInstance } from "@conic/instance";

const emit = defineEmits(["close", "update"]);

const props = defineProps<{
  show: boolean;
}>();

const defaultInstanceName = computed(() => {
  let modLoaderTypeText = "";
  switch (modLoaderType.value) {
    case 1:
      modLoaderTypeText = "quilt";
      break;
    case 2:
      modLoaderTypeText = "fabric";
      break;
    case 3:
      modLoaderTypeText = "neoforged";
      break;
    case 4:
      modLoaderTypeText = "forged";
      break;
  }
  return `${minecraftVersion.value ? minecraftVersion.value : "未命名配置"}${modLoaderTypeText ? "-" + modLoaderTypeText + modLoaderVersion.value : ""}`;
});

const instanceNameValue = ref("");

const minecraftVersion = ref("");
const modLoaderType = ref(0);
const modLoaderVersion = ref("");

const currentComponent = ref("settings");
const transitionName = ref("slide-left");

function back() {
  transitionName.value = "slide-right";
  currentComponent.value = "settings";
}

const setMinecraft = (versionId: string) => {
  minecraftVersion.value = versionId;
  back();
};
const setQuilt = (version: string) => {
  modLoaderVersion.value = version;
  back();
};
const setFabric = (version: string) => {
  modLoaderVersion.value = version;
  back();
};
const setForge = (version: string) => {
  modLoaderVersion.value = version;
  back();
};
const setNeoforged = (version: string) => {
  modLoaderVersion.value = version;
  back();
};

watch(modLoaderType, () => {
  modLoaderVersion.value = "";
});

watch(minecraftVersion, () => {
  modLoaderType.value = 0;
  modLoaderVersion.value = "";
  quiltVersionList.value = [];
  fabricVersionList.value = [];
  forgeVersionList.value = [];
});

const forgeVersionList = ref<ForgeVersionItem[]>([]);
const quiltVersionList = ref<QuiltVersion[]>([]);
const fabricVersionList = ref<FabricLoaderArtifact[]>([]);
const neoforgedVersionList = ref<string[]>([]);

const forgeIsLoading = ref(false);
const fabricIsLoading = ref(false);
const quiltIsLoading = ref(false);
const neoforgedIsLoading = ref(false);
const modLoaderListLoading = computed(() => {
  return (
    forgeIsLoading.value ||
    fabricIsLoading.value ||
    quiltIsLoading.value ||
    neoforgedIsLoading.value
  );
});

watchEffect(async () => {
  fabricIsLoading.value = true;
  if (minecraftVersion.value) {
    try {
      fabricVersionList.value = await getFabricVersionList(minecraftVersion.value);
    } catch {
      fabricVersionList.value = [];
    }
  }
  fabricIsLoading.value = false;
});
watchEffect(async () => {
  quiltIsLoading.value = true;
  if (minecraftVersion.value) {
    try {
      quiltVersionList.value = await getQuiltVersionList(minecraftVersion.value);
    } catch {
      quiltVersionList.value = [];
    }
  }
  quiltIsLoading.value = false;
});
watchEffect(async () => {
  forgeIsLoading.value = true;
  if (minecraftVersion.value) {
    try {
      forgeVersionList.value = await getForgeVersionList(minecraftVersion.value);
    } catch {
      forgeVersionList.value = [];
    }
  }
  forgeIsLoading.value = false;
});
watchEffect(async () => {
  neoforgedIsLoading.value = true;
  if (minecraftVersion.value) {
    try {
      neoforgedVersionList.value = await getNeoforgedVersionList(minecraftVersion.value);
    } catch {
      neoforgedVersionList.value = [];
    }
  }
  neoforgedIsLoading.value = false;
});

const disabledModLoaderId = computed(() => {
  const result = [];
  if (quiltVersionList.value.length === 0) {
    result.push(1);
  }
  if (fabricVersionList.value.length === 0) {
    result.push(2);
  }
  if (neoforgedVersionList.value.length === 0) {
    result.push(3);
  }
  if ((forgeVersionList.value, length === 0)) {
    result.push(4);
  }
  console.warn(result);
  return result;
});

const creating = ref(false);

const createInstance = () => {
  let parsedModLoaderType: "Quilt" | "Fabric" | "Neoforged" | "Forge" | undefined = undefined;
  switch (modLoaderType.value) {
    case 1:
      parsedModLoaderType = "Quilt";
      break;
    case 2:
      parsedModLoaderType = "Fabric";
      break;
    case 3:
      parsedModLoaderType = "Neoforged";
      break;
    case 4:
      parsedModLoaderType = "Forge";
      break;
  }
  creating.value = true;
  const newInstanceConfig = {
    name: instanceNameValue.value ? instanceNameValue.value : defaultInstanceName.value,
    runtime: {
      minecraft: minecraftVersion.value,
      mod_loader_type: parsedModLoaderType,
      mod_loader_version: parsedModLoaderType ? modLoaderVersion.value : undefined,
    },
    launch_config: {
      enable_instance_specific_settings: false,
    },
  };
  conicCreateInstance(newInstanceConfig)
    .then(() => {
      emit("update");
      close();
    })
    .catch(() => {
      emit("update");
      close();
    });
};

const close = () => {
  creating.value = false;
  minecraftVersion.value = "";
  modLoaderType.value = 0;
  modLoaderVersion.value = "";
  emit("close");
};
</script>

<style lang="less" scoped>
.create-instance {
  width: 100%;
  height: 100%;
  padding: 12px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;

  div.content {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }
}

.dialog-button {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  position: absolute;
  top: 4px;
  right: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  background: var(--close-btn-background);

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

  i {
    transition: all 100ms ease;
  }
}
</style>
