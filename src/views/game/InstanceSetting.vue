<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instance-settings">
    <ScrollView>
      <div class="title">
        <AppIcon name="settings"></AppIcon>
        <p>实例设置</p>
      </div>
      <div style="padding: 0 16px">
        <div class="instance">
          <p class="instance-name" @click="editInstanceName">
            <span v-if="!editingInstanceName">
              {{ currentInstance.config.name }}
            </span>
            <AppIcon v-if="!editingInstanceName" name="create-outline" :size="16"></AppIcon>
            <input
              type="text"
              ref="instance-name-input"
              v-model="currentInstance.config.name"
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
        <SettingGroup>
          <SettingItem
            title="设置背景图像"
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
                  await removeBackground(currentInstance.id);
                  await instanceStore.loadInstances();
                })()
              "
              >移除图像</BaseButton
            >
          </SettingItem>
          <SettingItem title="在启动器背景使用实例背景" :disabled="!currentInstance.has_background">
            <BaseSwitch></BaseSwitch>
          </SettingItem>
        </SettingGroup>
        <SettingGroup>
          <SettingItem
            title="Minecraft"
            icon="minecraft"
            :description="`已选择 ${currentInstance.config.runtime.minecraft}`"
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

        <setting-group>
          <setting-item title="启用实例独立设置" icon="settings">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings
              "></BaseSwitch>
          </setting-item>
        </setting-group>
        <setting-group :title="'启动选项'" :disabled="!enableInstanceSpecificSettings">
          <SettingItem :title="'窗口大小'" :description="'游戏窗口的初始大小'">
            <BaseInput
              width="100px"
              style="display: inline-block; margin-right: 16px"
              :placeholder="'宽'"
              :number-only="true"
              :disabled="config.launch.fullscreen"
              v-model.number="instanceStore.currentInstance.config.launch_config.width"
              :lazy-update-value="true">
            </BaseInput>
            <BaseInput
              width="100px"
              style="display: inline-block"
              :placeholder="'高'"
              :number-only="true"
              :disabled="config.launch.fullscreen"
              v-model.number="instanceStore.currentInstance.config.launch_config.height"
              :lazy-update-value="true">
            </BaseInput>
            <span style="font-size: 12px; margin-left: 8px">{{ "全屏" }}: </span>
            <BaseSwitch
              v-model="instanceStore.currentInstance.config.launch_config.fullscreen"></BaseSwitch>
          </SettingItem>
          <SettingItem :title="'启动游戏后退出启动器'">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.quit_app_after_launch
              "></BaseSwitch>
          </SettingItem>
          <SettingItem
            title="跳过游戏文件检查"
            description="启动游戏前启动器将不会尝试检查或补全游戏文件">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.skip_check_files
              "></BaseSwitch>
          </SettingItem>
        </setting-group>
        <SettingGroup :title="'内存'" :disabled="!enableInstanceSpecificSettings">
          <SettingItem :title="'自动分配内存'">
            <BaseSwitch
              v-model="instanceStore.currentInstance.config.launch_config.auto_memory"></BaseSwitch>
          </SettingItem>
          <SettingItem
            :title="'手动分配内存'"
            :description="'手动指定 Java 堆的最大大小，关闭自动分配后生效'"
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
          :title="'高级启动选项'"
          :resetable="advancedLaunchOptionsChanged"
          @reset="resetAdvanceOptions"
          :disabled="!enableInstanceSpecificSettings">
          <SettingItem :title="'Java 垃圾回收器'">
            <BaseSelect
              :display-name="['G1', 'Z', 'Parallel', 'Serial']"
              :options="['G1', 'Z', 'Parallel', 'Serial']"
              v-model="instanceStore.currentInstance.config.launch_config.gc"></BaseSelect>
          </SettingItem>
          <setting-item :title="'添加 JVM 参数'" :description="'将会放在默认 JVM 参数后面'">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.extra_jvm_args"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item :title="'添加游戏参数'" :description="'添加游戏需要的其他参数'">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.extra_mc_args"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item :title="'添加类路径'" :description="'Windows 下用分号隔开，其他系统用冒号'">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.extra_class_paths"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item :title="'启动前执行'" :description="'将会添加至启动命令的前一行'">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.execute_before_launch"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item :title="'包装命令'" :description="'将会添加至启动命令的开头'">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.wrap_command"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item :title="'启动后执行'" :description="'将会添加至启动脚本的最后一行'">
            <BaseInput
              width="300px"
              v-model="instanceStore.currentInstance.config.launch_config.execute_after_launch"
              :lazy-update-model="true">
            </BaseInput>
          </setting-item>
          <setting-item
            :title="'忽略无效的 Minecraft 凭证'"
            :description="'将 <code>-Dfml.ignoreInvalidMinecraftCertificates=true</code> 添加到 JVM 参数中'">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config
                  .ignore_invalid_minecraft_certificates
              ">
            </BaseSwitch>
          </setting-item>
          <setting-item
            :title="'忽略补丁差异'"
            :description="'将 <code>-Dfml.ignorePatchDiscrepancies=true</code> 添加到 JVM 参数中'">
            <BaseSwitch
              v-model="
                instanceStore.currentInstance.config.launch_config.ignore_patch_discrepancies
              ">
            </BaseSwitch>
          </setting-item>
        </SettingCollapse>

        <setting-group :danger="true">
          <setting-item
            title="删除实例"
            description="一旦删除此实例，该实例下的存档、材质包等一切游戏数据都将丢失"
            icon="trash"
            :navigable="true"
            @click="openDeleteInstanceDialog">
          </setting-item>
          <setting-item
            title="重置实例"
            description="一旦重置实例，该实例下的存档、材质包等一切游戏数据都将丢失，仅实例配置会被保留"
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
import BaseDropdownSelect from "@/components/BaseDropdownSelect.vue";
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

const instanceStore = useInstanceStore();
const dialogStore = useDialogStore();

const instanceName = computed(() => {
  return instanceStore.currentInstance.config.name;
});

const config = useConfigStore();

const enableInstanceSpecificSettings = computed(() => {
  return instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings;
});

let oldEnabledSpecificSettings =
  instanceStore.currentInstance.config.launch_config.enable_instance_specific_settings;

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
  if (currentInstance.value.has_background) {
    backgroundFileSrc.value = convertFileSrc(await getBackgroundPath(currentInstance.value.id));
  }
});

watch(
  () => currentInstance.value.has_background,
  async (newValue) => {
    if (newValue) {
      backgroundFileSrc.value = convertFileSrc(await getBackgroundPath(currentInstance.value.id));
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
  if (filePath) {
    console.log(filePath);
    await addBackgroundImage(filePath, currentInstance.value.id);
    await instanceStore.loadInstances();
  }
}

watchEffect(() => {
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
