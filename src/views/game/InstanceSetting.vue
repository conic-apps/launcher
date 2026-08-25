<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-settings">
    <ScrollView>
      <div class="title">
        <AppIcon name="settings"></AppIcon>
        <p>{{ t("game.instance.title") }}</p>
      </div>
      <div style="padding: 0 16px">
        <div class="instance" v-if="currentInstance">
          <p class="instance-name" @click="editInstanceName">
            <span v-if="!editingInstanceName">
              {{ currentInstance.config.name }}
            </span>
            <AppIcon v-if="!editingInstanceName" name="create-outline" :size="16"></AppIcon>
            <input
              type="text"
              ref="instance-name-input"
              v-model="currentInstance.config.name"
              autocapitalize="off"
              autocomplete="off"
              autocorrect="off"
              v-else
              @blur="editingInstanceName = false" />
          </p>
          <div class="details">
            <span
              :class="`tag ${currentInstance.config.runtime.mod_loader_type.toLowerCase()}`"
              v-if="currentInstance.config.runtime.mod_loader_type"
              >{{ currentInstance.config.runtime.mod_loader_type }}</span
            >
            <span class="tag vanilla" v-else>Vanilla</span>
            <span class="minecraft-version" v-if="currentInstance.config.runtime.minecraft"
              ><span class="label">Minecraft: </span
              ><span>{{ currentInstance.config.runtime.minecraft }}</span></span
            >
          </div>
          <img
            :src="backgroundFileSrc"
            v-if="currentInstance.has_background && backgroundFileSrc"
            alt=""
            class="background"
            v-show="imgLoaded"
            @load="imgLoaded = true"
            @error="imgLoaded = false" />
        </div>
        <SettingGroup v-if="currentInstance">
          <SettingItem
            :title="t('game.instance.setBackground')"
            description=""
            icon="image"
            :navigable="!currentInstance.has_background"
            @click="getBackground">
            <AppIcon
              name="chevron-forward"
              style="margin-right: 4px"
              v-if="!currentInstance.has_background"></AppIcon>
            <BaseButton
              color="var(--ctp-red)"
              v-else
              @click.stop="
                (async () => {
                  if (!currentInstance) {
                    throw 'currentInstance is null';
                  }
                  await removeBackground(currentInstance.id);
                  await instanceStore.loadInstances();
                })()
              "
              >{{ t("game.instance.removeImage") }}</BaseButton
            >
          </SettingItem>
          <SettingItem
            :title="t('game.instance.useAsLauncherBg')"
            :disabled="!currentInstance.has_background">
            <BaseSwitch
              v-if="instanceStore.currentInstance"
              v-model="
                instanceStore.currentInstance.config.use_as_launcher_background
              "></BaseSwitch>
          </SettingItem>
        </SettingGroup>
        <SettingGroup v-if="currentInstance">
          <SettingItem
            title="Minecraft"
            icon="minecraft"
            :description="
              t('game.instance.selected', { version: currentInstance.config.runtime.minecraft })
            "
            :navigable="true">
            <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon>
          </SettingItem>
          <!-- <SettingItem title="模组加载器" :disabled="modLoaderSettingDisabled"> -->
          <!--   <BaseSelect -->
          <!--     :options="['None', 'Fabric', 'Quilt', 'Neoforge', 'Forge']" -->
          <!--     :display-name="['无', 'Fabric', 'Quilt', 'Neoforge', 'Forge']" -->
          <!--     :disabled="modLoaderTypeDisabled" -->
          <!--     v-model="currentInstance.config.runtime.mod_loader_type"></BaseSelect> -->
          <!-- </SettingItem> -->
          <!-- <SettingItem -->
          <!--   title="加载器版本" -->
          <!--   icon="extension-puzzle" -->
          <!--   :description=" -->
          <!--     currentInstance.config.runtime.mod_loader_version -->
          <!--       ? `已选择 ${currentInstance.config.runtime.mod_loader_type} ${currentInstance.config.runtime.mod_loader_version}` -->
          <!--       : '点击以选择模组加载器版本' -->
          <!--   " -->
          <!--   :navigable="true" -->
          <!--   :disabled="!minecraftVersion || modLoaderType === 'None' || modLoaderSettingDisabled"> -->
          <!--   <span style="font-size: 14px; opacity: 0.8; margin-right: 8px">{{ -->
          <!--     minecraftVersion -->
          <!--   }}</span> -->
          <!--   <AppIcon name="chevron-forward" style="margin-right: 4px"></AppIcon> -->
          <!-- </SettingItem> -->
        </SettingGroup>

        <setting-group v-if="currentInstance && instanceStore.currentInstance">
          <setting-item :title="t('game.instance.enableInstanceSettings')" icon="settings">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings
              "></BaseSwitch>
          </setting-item>
        </setting-group>
        <setting-group
          :title="t('game.instance.launchOptions')"
          :disabled="!enableInstanceSpecificSettings"
          v-if="currentInstance && instanceStore.currentInstance">
          <SettingItem
            :title="t('game.instance.windowSize')"
            :description="t('game.instance.windowSizeDesc')">
            <BaseInput
              width="100px"
              style="display: inline-block; margin-right: 16px"
              :placeholder="t('game.instance.width')"
              :number-only="true"
              :disabled="config.launch.fullscreen"
              v-model.number="instanceStore.currentInstance.config.launch_config.width"
              :lazy-update-model="true">
            </BaseInput>
            <BaseInput
              width="100px"
              style="display: inline-block"
              :placeholder="t('game.instance.height')"
              :number-only="true"
              :disabled="config.launch.fullscreen"
              v-model.number="instanceStore.currentInstance.config.launch_config.height"
              :lazy-update-model="true">
            </BaseInput>
            <span style="font-size: 12px; margin-left: 8px"
              >{{ t("game.instance.fullscreen") }}:
            </span>
            <BaseSwitch
              v-model="instanceStore.currentInstance.config.launch_config.fullscreen"></BaseSwitch>
          </SettingItem>
          <SettingItem :title="t('game.instance.quitAfterLaunch')">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.quit_app_after_launch
              "></BaseSwitch>
          </SettingItem>
          <SettingItem
            :title="t('game.instance.skipFileCheck')"
            :description="t('game.instance.skipFileCheckDesc')">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.skip_check_files
              "></BaseSwitch>
          </SettingItem>
        </setting-group>
        <SettingGroup
          :title="t('game.instance.memory')"
          :disabled="!enableInstanceSpecificSettings"
          v-if="currentInstance && instanceStore.currentInstance">
          <SettingItem :title="t('game.instance.autoMemory')">
            <BaseSwitch
              v-model="instanceStore.currentInstance.config.launch_config.auto_memory"></BaseSwitch>
          </SettingItem>
          <SettingItem
            :title="t('game.instance.manualMemory')"
            :description="t('game.instance.manualMemoryDesc')"
            :disabled="instanceStore.currentInstance.config.launch_config.auto_memory">
            <BaseInput
              width="100px"
              style="display: inline-block; margin-right: 8px"
              :number-only="true"
              :disabled="instanceStore.currentInstance.config.launch_config.auto_memory"
              v-model.number="instanceStore.currentInstance.config.launch_config.max_memory"
              :lazy-update-model="true">
            </BaseInput>
            <span style="font-size: 12px">MB</span>
          </SettingItem>
        </SettingGroup>

        <SettingCollapse
          :title="t('game.instance.advancedOptions')"
          :resetable="advancedLaunchOptionsChanged"
          @reset="resetAdvanceOptions"
          :disabled="!enableInstanceSpecificSettings"
          v-if="currentInstance && instanceStore.currentInstance">
          <SettingItem :title="t('game.instance.gc')">
            <BaseSelect
              :display-name="['G1', 'Z', 'Parallel', 'Serial']"
              :options="['G1', 'Z', 'Parallel', 'Serial']"
              v-model="instanceStore.currentInstance.config.launch_config.gc"></BaseSelect>
          </SettingItem>
          <setting-item
            :title="t('game.instance.jvmArgs')"
            :description="t('game.instance.jvmArgsDesc')">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.extra_jvm_args"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="t('game.instance.gameArgs')"
            :description="t('game.instance.gameArgsDesc')">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.extra_mc_args"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="t('game.instance.classPath')"
            :description="t('game.instance.classPathDesc')">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.extra_class_paths"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="t('game.instance.beforeLaunch')"
            :description="t('game.instance.beforeLaunchDesc')">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.execute_before_launch"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="t('game.instance.wrapCommand')"
            :description="t('game.instance.wrapCommandDesc')">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.wrap_command"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="t('game.instance.afterLaunch')"
            :description="t('game.instance.afterLaunchDesc')">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.execute_after_launch"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="t('game.instance.ignoreInvalidCerts')"
            :description="t('game.instance.ignoreInvalidCertsDesc')">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config
                  .ignore_invalid_minecraft_certificates
              ">
            </BaseSwitch>
          </setting-item>
          <setting-item
            :title="t('game.instance.ignorePatchDiff')"
            :description="t('game.instance.ignorePatchDiffDesc')">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.ignore_patch_discrepancies
              ">
            </BaseSwitch>
          </setting-item>
        </SettingCollapse>

        <setting-group :danger="true">
          <setting-item
            :title="t('game.instance.deleteInstance')"
            :description="t('game.instance.deleteInstanceDesc')"
            icon="trash"
            :navigable="true"
            @click="openDeleteInstanceDialog">
          </setting-item>
          <setting-item
            :title="t('game.instance.resetInstance')"
            :description="t('game.instance.resetInstanceDesc')"
            icon="refresh"
            :navigable="true">
          </setting-item>
        </setting-group>
      </div>
    </ScrollView>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import BaseInput from "@/components/BaseInput.vue";
import { computed, nextTick, onMounted, ref, useTemplateRef, watch, watchEffect } from "vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { useInstanceStore } from "@/store/instance";
import BaseButton from "@/components/BaseButton.vue";
import {
  addBackgroundImage,
  getBackgroundPath,
  removeBackground,
  updateInstance,
} from "@conic/instance";
import { useDialogStore } from "@/store/dialog";
import { useInstanceSettings } from "./useGameView";
import ScrollView from "@/components/ScrollView.vue";
import AppIcon from "@/components/AppIcon.vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import BaseSelect from "@/components/BaseSelect.vue";
import SettingCollapse from "@/components/SettingCollapse.vue";
import { getDefaultConfig } from "@conic/config";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const instanceStore = useInstanceStore();
const dialogStore = useDialogStore();

const config = useConfigStore();

const enableInstanceSpecificSettings = computed(() => {
  return (
    instanceStore.currentInstance?.config.launch_config.enable_instance_specific_settings ?? false
  );
});

let oldEnabledSpecificSettings =
  instanceStore.currentInstance?.config.launch_config.enable_instance_specific_settings ?? false;

const currentInstance = computed(() => instanceStore.currentInstance);

const editingInstanceName = ref(false);
const instanceNameInput = useTemplateRef("instance-name-input");
async function editInstanceName() {
  editingInstanceName.value = true;
  await nextTick();
  if (instanceNameInput.value) {
    instanceNameInput.value.focus();
  }
}

const backgroundFileSrc = ref(null as string | null);
const imgLoaded = ref(false);

onMounted(async () => {
  if (!currentInstance.value) {
    throw "currentInstance is null";
  }
  if (currentInstance.value.has_background) {
    backgroundFileSrc.value =
      convertFileSrc(await getBackgroundPath(currentInstance.value.id)) + "?t=" + Date.now();
  }
});

watch(
  () => (currentInstance.value ? currentInstance.value.has_background : null),
  async (newValue) => {
    if (newValue && currentInstance.value) {
      backgroundFileSrc.value =
        convertFileSrc(await getBackgroundPath(currentInstance.value.id)) + "?t=" + Date.now();
    }
  },
);

async function getBackground() {
  const filePath = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif", "svg", "ico"],
      },
    ],
  });
  if (filePath && currentInstance.value) {
    console.log(filePath);
    await addBackgroundImage(filePath, currentInstance.value.id);
    await instanceStore.loadInstances();
  }
}

watchEffect(() => {
  if (!instanceStore.currentInstance) {
    return;
  }
  const currentInstanceConfig = instanceStore.currentInstance.config;
  document.body.classList.add("saving-instance-settings");
  if (
    currentInstanceConfig.launch_config.enable_instance_specific_settings &&
    !oldEnabledSpecificSettings
  ) {
    instanceStore.currentInstance.config.launch_config = {
      enable_instance_specific_settings: true,
      auto_memory: config.launch.auto_memory,
      max_memory: config.launch.max_memory,
      server:
        config.launch.server && config.launch.server.ip
          ? {
              ip: config.launch.server?.ip,
              port: config.launch.server?.port,
            }
          : undefined,
      width: config.launch.width,
      height: config.launch.height,
      fullscreen: config.launch.fullscreen,
      extra_jvm_args: config.launch.extra_jvm_args,
      extra_mc_args: config.launch.extra_mc_args,
      is_demo: config.launch.is_demo,
      ignore_invalid_minecraft_certificates: config.launch.ignore_invalid_minecraft_certificates,
      ignore_patch_discrepancies: config.launch.ignore_patch_discrepancies,
      extra_class_paths: config.launch.extra_class_paths,
      gc: config.launch.gc,
      launcher_name: config.launch.launcher_name,
      wrap_command: config.launch.wrap_command,
      execute_before_launch: config.launch.execute_before_launch,
      execute_after_launch: config.launch.execute_after_launch,
      skip_check_files: config.launch.skip_check_files,
      quit_app_after_launch: config.launch.quit_app_after_launch,
    };
    document.body.classList.remove("saving-instance-settings");
    oldEnabledSpecificSettings =
      currentInstanceConfig.launch_config.enable_instance_specific_settings;
  }
  if (
    !currentInstanceConfig.launch_config.enable_instance_specific_settings &&
    oldEnabledSpecificSettings
  ) {
    instanceStore.currentInstance.config.launch_config = {
      enable_instance_specific_settings: false,
    };
    document.body.classList.remove("saving-instance-settings");
    oldEnabledSpecificSettings =
      currentInstanceConfig.launch_config.enable_instance_specific_settings;
  }
  oldEnabledSpecificSettings =
    currentInstanceConfig.launch_config.enable_instance_specific_settings;
  updateInstance(currentInstanceConfig, instanceStore.currentInstance.id).then(() => {
    document.body.classList.remove("saving-instance-settings");
  });
});

const advancedLaunchOptionsChanged = computed(() => {
  if (!instanceStore.currentInstance) {
    return;
  }
  const launchOptions = instanceStore.currentInstance.config.launch_config;
  const isDefault =
    launchOptions.gc === "G1" &&
    !launchOptions.extra_jvm_args &&
    !launchOptions.extra_mc_args &&
    !launchOptions.extra_class_paths &&
    !launchOptions.execute_before_launch &&
    !launchOptions.wrap_command &&
    !launchOptions.execute_after_launch &&
    !launchOptions.ignore_invalid_minecraft_certificates &&
    !launchOptions.ignore_patch_discrepancies;
  return !isDefault;
});

async function resetAdvanceOptions() {
  if (!instanceStore.currentInstance) {
    return;
  }
  const defaultConfig = await getDefaultConfig();
  instanceStore.currentInstance.config.launch_config.gc = defaultConfig.launch.gc;
  instanceStore.currentInstance.config.launch_config.extra_jvm_args =
    defaultConfig.launch.extra_jvm_args;
  instanceStore.currentInstance.config.launch_config.extra_mc_args =
    defaultConfig.launch.extra_mc_args;
  instanceStore.currentInstance.config.launch_config.extra_class_paths =
    defaultConfig.launch.extra_class_paths;
  instanceStore.currentInstance.config.launch_config.execute_before_launch =
    defaultConfig.launch.execute_before_launch;
  instanceStore.currentInstance.config.launch_config.wrap_command =
    defaultConfig.launch.wrap_command;
  instanceStore.currentInstance.config.launch_config.execute_after_launch =
    defaultConfig.launch.execute_after_launch;
  instanceStore.currentInstance.config.launch_config.ignore_invalid_minecraft_certificates =
    defaultConfig.launch.ignore_invalid_minecraft_certificates;
  instanceStore.currentInstance.config.launch_config.ignore_patch_discrepancies =
    defaultConfig.launch.ignore_patch_discrepancies;
}

function openDeleteInstanceDialog() {
  if (!instanceStore.currentInstance) {
    return;
  }
  useInstanceSettings().value = false;
  dialogStore.confirmDeleteInstance.instanceToDelete = instanceStore.currentInstance;
  dialogStore.confirmDeleteInstance.visible = true;
}
</script>

<style lang="less" scoped>
.instance-settings {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  position: relative;

  .title {
    width: 100%;
    background: var(--ctp-mantle);
    height: 52px;
    padding: 0 32px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 8px;
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
