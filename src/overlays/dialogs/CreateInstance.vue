<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <BaseDialog :visible="dialogStore.createInstance.visible" :width="480" :height="480">
    <div class="create-instance">
      <div class="content">
        <p :style="{ width: '100%', paddingBottom: '16px', paddingLeft: '8px', paddingTop: '8px' }">
          {{ dialogTitle }}
        </p>
        <Transition mode="out-in" :name="transitionName">
          <div class="settings" v-if="currentComponent == 'settings'">
            <div class="instance">
              <p class="instance-name" @click="editInstanceName">
                <span v-if="!instanceNameEdit">
                  {{ !!customInstanceName ? customInstanceName : defaultInstanceName }}
                </span>
                <AppIcon v-if="!instanceNameEdit" name="create-outline" :size="16"></AppIcon>
                <input
                  type="text"
                  ref="instance-name-input"
                  v-model="customInstanceName"
                  autocapitalize="off"
                  autocomplete="off"
                  autocorrect="off"
                  :placeholder="defaultInstanceName"
                  v-else
                  @blur="instanceNameEdit = false" />
              </p>
              <div class="details">
                <span
                  :class="`tag ${modLoaderType.toLowerCase()}`"
                  v-if="modLoaderType != 'None'"
                  >{{ modLoaderType }}</span
                >
                <span class="tag vanilla" v-else>Vanilla</span>
                <span class="minecraft-version" v-if="minecraftVersion"
                  ><span class="label">Minecraft: </span><span>{{ minecraftVersion }}</span></span
                >
              </div>
              <img
                :src="backgroundFileSrc"
                alt=""
                class="background"
                v-show="imgLoaded"
                @load="imgLoaded = true"
                @error="imgLoaded = false" />
            </div>
            <SettingGroup>
              <SettingItem
                :title="t('game.instance.setBackground')"
                description=""
                icon="image"
                :navigable="backgroundFilePath === null"
                @click="getBackground">
                <AppIcon
                  name="chevron-forward"
                  style="margin-right: 4px"
                  v-if="backgroundFilePath === null"></AppIcon>
                <BaseButton
                  color="var(--ctp-red)"
                  v-else
                  @click.stop="
                    () => {
                      backgroundFilePath = null;
                    }
                  "
                  >{{ t("game.instance.removeImage") }}</BaseButton
                >
              </SettingItem>
              <SettingItem
                :title="t('game.instance.useAsLauncherBg')"
                :disabled="backgroundFilePath === null">
                <BaseSwitch></BaseSwitch>
              </SettingItem>
            </SettingGroup>
            <SettingGroup :title="t('overlays.dialogs.createInstance.versionSettings')">
              <SettingItem
                title="Minecraft"
                icon="minecraft"
                :description="
                  minecraftVersion ? t('game.instance.selected', { version: minecraftVersion }) : ''
                "
                :navigable="true"
                @click="switchComponent('minecraft-choose')">
                <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
              </SettingItem>
              <SettingItem
                :title="t('overlays.dialogs.createInstance.modLoaderType')"
                :loading="modLoaderSettingLoading">
                <BaseSelect
                  :options="['None', 'Fabric', 'Quilt', 'Neoforge', 'Forge']"
                  :display-name="[
                    t('overlays.dialogs.createInstance.modLoaderNone'),
                    'Fabric',
                    'Quilt',
                    'Neoforge',
                    'Forge',
                  ]"
                  :disabled="modLoaderTypeDisabled"
                  v-model="modLoaderType"></BaseSelect>
              </SettingItem>
              <SettingItem
                :title="t('overlays.dialogs.createInstance.loaderVersion')"
                icon="extension-puzzle"
                :description="
                  minecraftVersion
                    ? modLoaderVersion
                      ? t('game.instance.selected', {
                          version: `${modLoaderType} ${modLoaderVersion}`,
                        })
                      : t('overlays.dialogs.createInstance.selectLoaderHint')
                    : t('overlays.dialogs.createInstance.needMinecraftFirst')
                "
                :navigable="true"
                @click="switchComponent('mod-loader-choose')"
                :disabled="!minecraftVersion || modLoaderType === 'None'">
                <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{
                  minecraftVersion
                }}</span>
                <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
              </SettingItem>
            </SettingGroup>
            <div style="display: flex; padding: 0 8px; gap: 12px">
              <BaseButton @click="close">{{
                t("overlays.dialogs.createInstance.cancel")
              }}</BaseButton>
              <BaseButton
                @click="confirmCreate"
                :disabled="
                  creating || !minecraftVersion || (modLoaderType != 'None' && !modLoaderVersion)
                ">
                {{
                  creating
                    ? t("overlays.dialogs.createInstance.creating")
                    : t("overlays.dialogs.createInstance.create")
                }}
              </BaseButton>
            </div>
          </div>
          <MinecraftChoose
            v-else-if="currentComponent === 'minecraft-choose'"
            v-model="minecraftVersion"
            @back="switchComponent('settings')"></MinecraftChoose>
          <ModLoaderChoose
            v-else-if="currentComponent === 'mod-loader-choose'"
            v-model:type="modLoaderType"
            v-model:version="modLoaderVersion"
            :fabricVersions="fabricVersions"
            :quiltVersions="quiltVersions"
            :neoforgeVersions="neoforgeVersions"
            :forgeVersions="forgeVersions"
            :modLoaderType="modLoaderType"
            @back="switchComponent('settings')"
            :minecraft="minecraftVersion"></ModLoaderChoose>
        </Transition>
      </div>
    </div>
  </BaseDialog>
</template>

<script setup lang="ts">
import BaseDialog from "@/components/BaseDialog.vue";
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import BaseButton from "@/components/BaseButton.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import MinecraftChoose from "./create/MinecraftChoose.vue";
import { useDialogStore } from "@/store/dialog";
import { computed, nextTick, ref, useTemplateRef, watch } from "vue";
import ModLoaderChoose from "./create/ModLoaderChoose.vue";
import BaseSelect from "@/components/BaseSelect.vue";
import {
  FabricLoaderArtifact,
  filterNeoforgeVersionList,
  getFabricVersionList,
  getForgeVersionList,
  getNeoforgeVersionList,
  getQuiltVersionList,
  QuiltVersion,
} from "@conic/install";
import { addBackgroundImage, createInstance } from "@conic/instance";
import { useInstanceStore } from "@/store/instance";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const dialogStore = useDialogStore();
type ComponentName = "settings" | "minecraft-choose" | "mod-loader-choose";
const currentComponent = ref<ComponentName>("minecraft-choose");
const transitionName = ref<"zoom-in" | "zoom-out">("zoom-in");

function switchComponent(componentName: ComponentName) {
  const oldValue = currentComponent.value;
  if (componentName === "minecraft-choose") {
    transitionName.value = "zoom-in";
  } else if (componentName === "settings" && oldValue === "minecraft-choose") {
    transitionName.value = "zoom-out";
  } else if (componentName === "settings" && oldValue === "mod-loader-choose") {
    transitionName.value = "zoom-in";
  } else if (componentName === "mod-loader-choose") {
    transitionName.value = "zoom-out";
  }
  currentComponent.value = componentName;
}

const defaultInstanceName = computed(() => {
  return `${minecraftVersion.value ? minecraftVersion.value : t("overlays.dialogs.createInstance.unnamedConfig")}${modLoaderType.value.toLowerCase() === "none" ? "" : "-" + modLoaderType.value.toLowerCase() + modLoaderVersion.value}`;
});
const instanceNameEdit = ref(false);
const customInstanceName = ref("");
const instanceNameInput = useTemplateRef("instance-name-input");
async function editInstanceName() {
  instanceNameEdit.value = true;
  await nextTick();
  if (instanceNameInput.value) {
    instanceNameInput.value.focus();
  }
}
const minecraftVersion = ref("");

watch(
  () => dialogStore.createInstance.visible,
  (newValue) => {
    if (newValue) {
      switchComponent("minecraft-choose");
    } else {
      // Clean
      customInstanceName.value = "";
    }
  },
);

const creating = ref(false);

const dialogTitle = computed(() => {
  if (currentComponent.value === "minecraft-choose") {
    return t("overlays.dialogs.createInstance.chooseMinecraftTitle");
  } else if (currentComponent.value === "mod-loader-choose") {
    return t("overlays.dialogs.createInstance.chooseModLoaderTitle");
  } else {
    return t("overlays.dialogs.createInstance.title");
  }
});

const modLoaderType = ref<"None" | "Quilt" | "Fabric" | "Neoforge" | "Forge">("None");
const modLoaderVersion = ref("");

const instanceStore = useInstanceStore();

async function confirmCreate() {
  creating.value = true;
  const newInstanceConfig = {
    name: customInstanceName.value ? customInstanceName.value : defaultInstanceName.value,
    runtime: {
      minecraft: minecraftVersion.value,
      mod_loader_type: modLoaderType.value == "None" ? undefined : modLoaderType.value,
      mod_loader_version: modLoaderType.value == "None" ? undefined : modLoaderVersion.value,
    },
    launch_config: {
      enable_instance_specific_settings: false,
    },
  };
  try {
    await instanceStore.loadInstances();
    const baseInstanceId = newInstanceConfig.name;
    let instanceIdAppendNumber = 0;
    while (
      instanceStore.instances.find(
        (instance) =>
          instance.id ===
          baseInstanceId +
            (instanceIdAppendNumber > 0 ? " " : "") +
            (instanceIdAppendNumber > 0 ? instanceIdAppendNumber : ""),
      )
    ) {
      instanceIdAppendNumber++;
    }
    const instanceId =
      baseInstanceId +
      (instanceIdAppendNumber > 0 ? " " : "") +
      (instanceIdAppendNumber > 0 ? instanceIdAppendNumber : "");
    await createInstance(newInstanceConfig, instanceId);
    if (backgroundFilePath.value) {
      await addBackgroundImage(backgroundFilePath.value, instanceId);
    }
  } catch (error) {
    console.error(error);
  } finally {
    instanceStore.loadInstances();
    close();
  }
}

function close() {
  minecraftVersion.value = "";
  modLoaderType.value = "None";
  modLoaderVersion.value = "";
  backgroundFilePath.value = null;
  creating.value = false;
  dialogStore.createInstance.visible = false;
}

const fabricVersions = ref<FabricLoaderArtifact[] | null>(null);
const quiltVersions = ref<QuiltVersion[] | null>(null);
const neoforgeVersions = ref<string[] | null>(null);
const forgeVersions = ref<string[] | null>(null);

watch(minecraftVersion, () => {
  updateModLoaderVersions();
});

watch(modLoaderType, () => {
  modLoaderVersion.value = "";
});

const modLoaderLoading = ref({
  fabric: true,
  quilt: true,
  forge: true,
  neoforge: true,
});

const modLoaderSettingLoading = computed(() => {
  return !!Object.values(modLoaderLoading.value).filter((value) => value).length;
});

const modLoaderAvailable = ref({
  fabric: false,
  quilt: false,
  forge: false,
  neoforge: false,
});

const modLoaderTypeDisabled = computed(() => {
  return Object.entries(modLoaderAvailable.value)
    .filter(([, value]) => !value)
    .map(([key]) => key.charAt(0).toUpperCase() + key.slice(1));
});

function updateModLoaderVersions() {
  for (const key of Object.keys(modLoaderLoading.value) as Array<
    keyof typeof modLoaderLoading.value
  >) {
    modLoaderLoading.value[key] = true;
  }
  for (const key of Object.keys(modLoaderAvailable.value) as Array<
    keyof typeof modLoaderAvailable.value
  >) {
    modLoaderAvailable.value[key] = false;
  }
  getFabricVersionList(minecraftVersion.value)
    .then((response) => {
      if (response.length > 0) {
        fabricVersions.value = response;
        modLoaderAvailable.value.fabric = true;
      } else {
        modLoaderAvailable.value.fabric = false;
      }
    })
    .catch((e) => {
      console.error(e);
      modLoaderAvailable.value.fabric = false;
    })
    .finally(() => {
      modLoaderLoading.value.fabric = false;
    });
  getQuiltVersionList(minecraftVersion.value)
    .then((response) => {
      if (response.length > 0) {
        quiltVersions.value = response;
        modLoaderAvailable.value.quilt = true;
      } else {
        modLoaderAvailable.value.quilt = false;
      }
    })
    .catch((e) => {
      console.error(e);
      modLoaderAvailable.value.quilt = false;
    })
    .finally(() => {
      modLoaderLoading.value.quilt = false;
    });
  getForgeVersionList()
    .then((response) => {
      if (response[minecraftVersion.value]) {
        forgeVersions.value = response[minecraftVersion.value];
        modLoaderAvailable.value.forge = true;
      } else {
        modLoaderAvailable.value.forge = false;
      }
    })
    .catch((e) => {
      console.error(e);
      modLoaderAvailable.value.forge = false;
    })
    .finally(() => {
      modLoaderLoading.value.forge = false;
    });
  getNeoforgeVersionList()
    .then((response) => {
      const filteredVersionList = filterNeoforgeVersionList(minecraftVersion.value, response);
      if (filteredVersionList.length > 0) {
        neoforgeVersions.value = filteredVersionList;
        modLoaderAvailable.value.neoforge = true;
      } else {
        modLoaderAvailable.value.neoforge = false;
      }
    })
    .catch((e) => {
      console.error(e);
      modLoaderAvailable.value.neoforge = false;
    })
    .finally(() => {
      modLoaderLoading.value.neoforge = false;
    });
}

const backgroundFilePath = ref<null | string>(null);

async function getBackground() {
  const filePath = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: t("overlays.dialogs.createInstance.imagesFilter"),
        extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg", "ico"],
      },
    ],
  });
  if (filePath) {
    backgroundFilePath.value = filePath;
  }
}

const backgroundFileSrc = computed(() => {
  return convertFileSrc(backgroundFilePath.value ?? "");
});

const imgLoaded = ref(false);
</script>

<style lang="less" scoped>
.create-instance {
  width: 100%;
  height: 100%;
  overflow: hidden;
  display: flex;
  padding: 4px;
  flex-direction: column;
  align-items: center;
  position: relative;

  div.content {
    width: 100%;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .instance {
    position: relative;
    border: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);
    border-left: 16px solid rgba(var(--ctp-lavender-rgb), 0.8);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    padding: 8px 16px;
    border-radius: 8px;
    width: calc(100% - 16px);
    margin: 4px 8px 12px 8px;
    height: 60px;
    transition:
      border-left 200ms ease,
      margin 200ms ease;
    img.background {
      mask-image: linear-gradient(to left, black 0%, transparent 100%);
      width: calc(100% - 100px);
      height: 100%;
      object-fit: cover;
      position: absolute;
      top: 0;
      right: 0;
      border-radius: 0 8px 8px 0;
    }
    p.instance-name {
      font-size: 16px;
      display: flex;
      align-items: center;
      height: 16px;
      span {
        max-width: calc(100% - 30px);
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
      }
      svg {
        margin-left: 4px;
        &:hover {
          transform: scale(1.02);
        }
        &:active {
          transform: scale(0.97);
        }
      }
      input {
        appearance: none;
        border: none;
        background: none;
        height: 100%;
        font-size: 16px;
        width: 100%;
      }
    }
    .details {
      margin-top: 6px;
      .tag {
        font-size: 11px;
        border-radius: 100px;
        padding: 1px 6px;
        font-weight: 500;
        display: inline-flex;
        align-items: center;
        width: fit-content;
      }
      .tag.quilt {
        background: var(--ctp-mauve);
        color: var(--ctp-text-inverse);
      }
      .tag.fabric {
        background: var(--ctp-yellow);
        color: var(--ctp-text-inverse);
      }
      .tag.forge {
        background: var(--ctp-blue);
        color: var(--ctp-text-inverse);
      }
      .tag.neoforge {
        background: var(--ctp-peach);
        color: var(--ctp-text-inverse);
      }
      .tag.vanilla {
        background: var(--ctp-green);
        color: var(--ctp-text-inverse);
      }
      .minecraft-version {
        font-size: 11px;
        margin-left: 8px;
        font-weight: 500;
        .label {
          opacity: 0.8;
          font-weight: 300;
        }
      }
    }
  }
}
</style>
