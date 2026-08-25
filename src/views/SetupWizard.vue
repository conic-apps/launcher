<template>
  <div class="setup-wizard">
    <div class="header-wrapper">
      <div class="header">
        <p class="title">首次启动设置</p>
        <p class="description">调整 Conic Launcher 以更贴合你的习惯</p>
        <button class="close" @click="navigationStore.navigate('game')">
          <AppIcon name="xmark" :size="16"></AppIcon>
        </button>
      </div>
    </div>
    <div class="body">
      <ScrollView>
        <Transition :name="transitionName" mode="out-in">
          <div class="language-setting" v-if="currentPhase === 'language'">
            <p class="title">欢迎</p>
            <p class="message">欢迎来到首次启动设置指南页！</p>
            <p class="message">
              Conic Launcher 是可高度自定义的启动器，但那些复杂的设置可能会让你不知所措。
            </p>
            <p class="message">
              这个指南将快速带你完成一些重要的设置项，以便待会儿你能够快速上手并启动你的游戏。
            </p>
            <div class="language-setting-container">
              <button
                class="language-item"
                v-for="(displayName, option) in supportedLanguages"
                :class="{ selected: configStore.language === option }"
                @click="configStore.language = option">
                {{ displayName }}
              </button>
            </div>
          </div>
          <div class="choose-palette" v-else-if="currentPhase === 'palette'">
            <p class="title">选择配色方案</p>
            <p class="message">Conic Launcher 支持多种配色方案，你可以按自己的喜好选择一个。</p>
            <div class="palette-setting-container">
              <SettingGroup style="width: 100%; margin-top: 16px">
                <SettingItem
                  title="跟随系统深色设置"
                  description="如果系统设置中设置为浅色，则使用 Latte ，否则使用 Mocha"
                  icon="moon"
                  icon-fill="none">
                  <BaseSwitch v-model="configStore.appearance.palette_follow_system"></BaseSwitch>
                </SettingItem>
                <div
                  class="color-style"
                  :class="{
                    'color-style-disabled': configStore.appearance.palette_follow_system,
                  }">
                  <div
                    :class="{ latte: true, selected: currentTheme == Palette.Latte }"
                    @click="choosePalette(Palette.Latte)">
                    <p>Latte</p>
                  </div>
                  <div
                    :class="{ frappe: true, selected: currentTheme == Palette.Frappe }"
                    @click="choosePalette(Palette.Frappe)">
                    <p>Frappé</p>
                  </div>
                  <div
                    :class="{ macchiato: true, selected: currentTheme == Palette.Macchiato }"
                    @click="choosePalette(Palette.Macchiato)">
                    <p>Macchiato</p>
                  </div>
                  <div
                    :class="{ mocha: true, selected: currentTheme == Palette.Mocha }"
                    @click="choosePalette(Palette.Mocha)">
                    <p>Mocha</p>
                  </div>
                </div>
              </SettingGroup>
            </div>
          </div>
          <div class="add-account" v-else-if="currentPhase === 'add-account'">
            <div class="account-add-container" v-if="!configStore.current_account">
              <AccountAdd style="width: 100%; height: 100%"></AccountAdd>
            </div>
            <div class="account-view-container" v-else>
              <p class="title">帐户已添加！</p>
              <p class="message">
                以下档案已设为默认档案。当你没有给实例设置要使用的档案时，启动器会使用此档案登录游戏
              </p>
              <div class="profile-card">
                <AccountAvatar
                  v-if="configStore.current_account"
                  ref="avatar"
                  :skin="accountSkin"
                  :uuid="accountUuid"
                  :size="56"
                  class="avatar"></AccountAvatar>
                <p class="profile-name">
                  {{ accountProfileName }}
                </p>
                <p
                  class="profile-type microsoft"
                  v-if="configStore.current_account.type === 'Microsoft'">
                  微软（正版帐户）
                </p>
                <p
                  class="profile-type yggdrasil"
                  v-else-if="configStore.current_account.type === 'Yggdrasil'">
                  Yggdrasil（外置登录）
                </p>
                <p
                  class="profile-type offline"
                  v-else="configStore.current_account.type === 'Offline'">
                  无认证服务（离线帐户）
                </p>
              </div>
            </div>
          </div>
          <div class="java-settings" v-else-if="currentPhase === 'java-settings'">
            <ScrollView>
              <p class="title">Java 虚拟机设置</p>
              <p class="message" v-if="isSupportedJVMAutoInstallPlatform">
                Conic Launcher 能够极大地简化 Java 环境的配置。启用「自动安装 Java
                运行环境」即可免于手动安装它们！
              </p>
              <p class="message warn" v-else>
                注意：当前平台可能无法自动安装 Java 运行环境，建议手动安装并禁用「自动安装 Java
                运行环境」
              </p>
              <SettingsJVM style="margin-top: 16px"></SettingsJVM>
            </ScrollView>
          </div>
          <div class="game-settings" v-else-if="currentPhase === 'launch-options'">
            <p class="title">游戏启动选项</p>
            <p class="message">
              这里列出了一些基本的启动选项。稍后你还可以在设置和实例独立设置中更改更多选项。
            </p>
            <SettingsGame style="margin-top: 16px"></SettingsGame>
          </div>
          <div class="import-instances" v-else-if="currentPhase === 'import-instances'">
            <p class="title">导入实例</p>
            <p class="message">你可以从其他启动器导入先前游玩的实例以便快速开始游戏</p>
            <div style="width: 100%; display: flex; justify-content: center; margin: 12px 0">
              <button class="import-from-other-launcher">导入实例</button>
            </div>
            <p class="message">
              如果你第一次接触 Minecraft，你也可以选择以最新 Minecraft 版本立刻创建空白实例
            </p>
            <div
              style="
                width: 100%;
                display: flex;
                justify-content: center;
                gap: 16px;
                margin: 12px 0;
              ">
              <button
                class="create-latest-instance"
                :class="{ creating: creatingLatestRelease, error: createLatestReleaseErrorOccured }"
                @click="createLatestReleaseInstance">
                {{ createLatestReleaseButtonText }}
              </button>
              <button
                class="create-latest-instance"
                :class="{
                  creating: creatingLatestSnapshot,
                  error: createLatestSnapshotErrorOccured,
                }"
                @click="createLatestSnapshotInstance">
                {{ createLatestSnapshotButtonText }}
              </button>
            </div>
          </div>
        </Transition>
      </ScrollView>
    </div>
    <div class="footer">
      <button class="back" :class="{ disabled: currentPhaseIndex === 0 }" @click="backPhase">
        <AppIcon name="chevron-back" :size="20"></AppIcon>
        返回
      </button>
      <button class="next" @click="nextPhase">
        {{ nextPhaseTexts[currentPhaseIndex] }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppIcon from "@/components/AppIcon.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import SettingItem from "@/components/SettingItem.vue";
import { useConfigStore } from "@/store/config";
import { useNavigationStore } from "@/store/navigation";
import { reloadPalette } from "@/theme";
import { Palette } from "@conic/config";
import { computed, onMounted, ref, watch } from "vue";
import AccountAdd from "./accounts/AccountAdd.vue";
import AccountAvatar from "@/components/AccountAvatar.vue";
import { yggdrasilGetSkinUrl } from "@conic/account";
import SettingsJVM from "./settings/SettingsJVM.vue";
import SettingsGame from "./settings/SettingsGame.vue";
import { createInstance } from "@conic/instance";
import { getMinecrafVersionManifest } from "@conic/install";
import ScrollView from "@/components/ScrollView.vue";

const navigationStore = useNavigationStore();
const configStore = useConfigStore();

const transitionName = ref<"slide-left" | "slide-right">("slide-left");
const currentPhaseIndex = ref(0);
const phases = [
  "language",
  "palette",
  "add-account",
  "java-settings",
  "launch-options",
  "import-instances",
] as const;
const currentPhase = computed(() => phases[currentPhaseIndex.value]);
const nextPhaseTexts = [
  "让我们开始吧！",
  "下一个（添加帐户）",
  "下一个（Java 虚拟机设置）",
  "下一个（游戏启动选项）",
  "下一个（导入实例）",
  "完成",
];

function backPhase() {
  if (currentPhaseIndex.value > 0) {
    transitionName.value = "slide-right";
    currentPhaseIndex.value--;
  }
}

function nextPhase() {
  if (currentPhaseIndex.value < phases.length - 1) {
    transitionName.value = "slide-left";
    currentPhaseIndex.value++;
  } else {
    navigationStore.navigate("game");
  }
}

const supportedLanguages = {
  en_us: "English",
  zh_cn: "简体中文",
  zh_tw: "繁體中文",
  ja_jp: "日本語",
  ko_kr: "한국어",
  de_de: "Deutsch",
  fr_fr: "Français",
  es_es: "Español",
  pt_br: "Português (Brasil)",
  ru_ru: "Русский",
  tr_tr: "Türkçe",
  pl_pl: "Polski",
};

const currentTheme = ref<Palette>(configStore.appearance.palette);

const choosePalette = (palette: Palette) => {
  configStore.appearance.palette = palette;
  currentTheme.value = palette;
  reloadPalette(
    {
      palette,
      paletteFollowSystem: configStore.appearance.palette_follow_system,
    },
    configStore.accessibility.high_contrast_mode,
  );
};

watch(
  () => configStore.appearance.palette_follow_system,
  (paletteFollowSystem) => {
    reloadPalette(
      {
        palette: configStore.appearance.palette,
        paletteFollowSystem,
      },
      configStore.accessibility.high_contrast_mode,
    );
    if (paletteFollowSystem) {
      if (matchMedia.matches) {
        currentTheme.value = Palette.Mocha;
      } else {
        currentTheme.value = Palette.Latte;
      }
    } else {
      currentTheme.value = configStore.appearance.palette;
    }
  },
);
const matchMedia = window.matchMedia("(prefers-color-scheme: dark)");
matchMedia.addEventListener("change", (event) => {
  if (configStore.appearance.palette_follow_system) {
    if (event.matches) {
      currentTheme.value = Palette.Mocha;
    } else {
      currentTheme.value = Palette.Latte;
    }
  } else {
    currentTheme.value = configStore.appearance.palette;
  }
  reloadPalette(
    {
      palette: configStore.appearance.palette,
      paletteFollowSystem: configStore.appearance.palette_follow_system,
    },
    configStore.accessibility.high_contrast_mode,
  );
});
if (configStore.appearance.palette_follow_system) {
  if (matchMedia.matches) {
    currentTheme.value = Palette.Mocha;
  } else {
    currentTheme.value = Palette.Latte;
  }
}

const accountSkin = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.skins.length > 0
      ? configStore.current_account.data.profile.skins[0].url
      : undefined;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return yggdrasilGetSkinUrl(configStore.current_account.data.profile);
  } else if (configStore.current_account?.type === "Offline") {
    return configStore.current_account.data.skin;
  } else {
    return undefined;
  }
});

const accountUuid = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.uuid;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.id;
  } else {
    return configStore.current_account ? configStore.current_account.data.uuid : "";
  }
});

const accountProfileName = computed(() => {
  if (configStore.current_account?.type === "Microsoft") {
    return configStore.current_account.data.profile.profile_name;
  } else if (configStore.current_account?.type === "Yggdrasil") {
    return configStore.current_account.data.profile.name;
  } else {
    return configStore.current_account ? configStore.current_account.data.name : "";
  }
});

const isSupportedJVMAutoInstallPlatform =
  ((window.__PLATFORM__.arch === "X64" || window.__PLATFORM__.arch === "Aarch64") &&
    (window.__PLATFORM__.os_family === "Windows" || window.__PLATFORM__.os_family === "Macos")) ||
  (window.__PLATFORM__.os_family === "Linux" && window.__PLATFORM__.arch === "X64");

onMounted(() => {
  if (!isSupportedJVMAutoInstallPlatform) {
    configStore.prefer_mojang_java = false;
  }
});

const createdLatestRelease = ref(null as null | string);
const createdLatestSnapshot = ref(null as null | string);
const creatingLatestRelease = ref(false);
const creatingLatestSnapshot = ref(false);
const createLatestReleaseErrorOccured = ref(false);
const createLatestSnapshotErrorOccured = ref(false);

async function createLatestReleaseInstance() {
  creatingLatestRelease.value = true;
  try {
    const minecraftVersionManifest = await getMinecrafVersionManifest();
    await createInstance({
      launch_config: { enable_instance_specific_settings: false },
      name: "最新版本",
      runtime: { minecraft: minecraftVersionManifest.latest.release },
    });
    createdLatestRelease.value = minecraftVersionManifest.latest.release;
  } catch (error) {
    console.log("Failed to create latest release instance", error);
    createLatestReleaseErrorOccured.value = true;
  } finally {
    creatingLatestRelease.value = false;
  }
}

async function createLatestSnapshotInstance() {
  creatingLatestSnapshot.value = true;
  try {
    const minecraftVersionManifest = await getMinecrafVersionManifest();
    await createInstance({
      launch_config: { enable_instance_specific_settings: false },
      name: "最新快照",
      runtime: { minecraft: minecraftVersionManifest.latest.snapshot },
    });
    createdLatestSnapshot.value = minecraftVersionManifest.latest.snapshot;
  } catch (error) {
    console.log("Failed to create latest snapshot instance", error);
    createLatestSnapshotErrorOccured.value = true;
  } finally {
    creatingLatestSnapshot.value = false;
  }
}

const createLatestReleaseButtonText = computed(() => {
  if (createLatestReleaseErrorOccured.value) {
    return "出现错误";
  } else if (creatingLatestRelease.value) {
    return "正在创建...";
  } else {
    return createdLatestRelease.value ?? "以最新正式版创建实例";
  }
});

const createLatestSnapshotButtonText = computed(() => {
  if (createLatestSnapshotErrorOccured.value) {
    return "出现错误";
  } else if (creatingLatestSnapshot.value) {
    return "正在创建...";
  } else {
    return createdLatestSnapshot.value ?? "以最新快照版创建实例";
  }
});
</script>

<style lang="less" scoped>
.setup-wizard {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;

  .header-wrapper {
    background: var(--ctp-base);
    border-radius: 0 0 12px 12px;
    width: calc(100% - 100px);
    height: fit-content;
    padding-bottom: 8px;
  }
  .header {
    background: rgba(var(--ctp-mauve-rgb), 0.2);
    border: 1px solid rgba(var(--ctp-mauve-rgb), 0.2);
    padding: 16px 32px;
    border-radius: 0 0 12px 12px;
    position: relative;
    .title {
      font-size: 16px;
    }
    .description {
      font-size: 12px;
      margin-top: 4px;
    }
    .close {
      position: absolute;
      right: 32px;
      top: 16px;
      appearance: none;
      border: none;
      background: none;
      padding: 2px;
      border-radius: 100px;
      transition: background 100ms ease;
      &:hover {
        background: rgba(var(--ctp-mauve-rgb), 0.4);
      }

      &:active {
        background: rgba(var(--ctp-mauve-rgb), 0.5);
      }
    }
  }
  .body {
    width: calc(100% - 186px);
    height: calc(100% - 192px);
    padding: 24px 36px;
    background: rgba(var(--ctp-base-rgb), 0.9);
    border-radius: 12px;
    margin-top: 32px;
    overflow: hidden;
    position: relative;

    .title {
      color: var(--ctp-lavender);
      font-size: 22px;
      margin-bottom: 8px;
    }
    .message {
      font-size: 13px;
      line-height: 1.5;
      width: 100%;
    }
    .message.warn {
      border: 1px solid var(--ctp-yellow);
      border-radius: 4px;
      padding: 4px 8px;
      background: rgba(var(--ctp-yellow-rgb), 0.1);
    }
  }

  .footer {
    width: 100%;
    height: 48px;
    background: rgba(var(--ctp-surface0-rgb), 0.6);
    backdrop-filter: blur(4px);
    position: absolute;
    bottom: 0;
    left: 0;
    display: flex;
    gap: 32px;
    padding: 16px;
    button {
      font-size: 13px;
      margin-top: -24px;
      height: 40px;
      appearance: none;
      border: none;
      background: none;
      color: var(--ctp-text-inverse);
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 4px;
      &:hover {
        transition: none;
      }
      &,
      &:active {
        transition: background 100ms ease;
      }
    }
    button.disabled {
      opacity: 0.7;
      pointer-events: none;
    }
    .back {
      width: 200px;
      background-image: linear-gradient(var(--ctp-blue), var(--ctp-blue));
      background: var(--ctp-blue);
      svg :deep(path) {
        stroke: var(--ctp-text-inverse);
      }
      &:hover {
        background-image:
          linear-gradient(#ffffff2f, #ffffff2f), linear-gradient(var(--ctp-blue), var(--ctp-blue));
      }
      &:active {
        background-image:
          linear-gradient(#ffffff6f, #ffffff6f), linear-gradient(var(--ctp-blue), var(--ctp-blue));
      }
    }
    .next {
      flex: 1;
      background: var(--ctp-mauve);
      &:hover {
        background-image:
          linear-gradient(#ffffff2f, #ffffff2f), linear-gradient(var(--ctp-mauve), var(--ctp-mauve));
      }
      &:active {
        background-image:
          linear-gradient(#ffffff6f, #ffffff6f), linear-gradient(var(--ctp-mauve), var(--ctp-mauve));
      }
    }
  }

  .language-setting-container {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: 8px;
    margin-top: 16px;
    button.language-item {
      height: 42px;
      display: flex;
      align-items: center;
      justify-content: center;
      border: none;
      background: none;
      border-radius: 8px;
      transition: background 100ms ease;
      &:hover {
        background: var(--ctp-surface0);
        transition: none;
      }
      &:active {
        background: var(--ctp-surface1);
        transition: background 200ms ease;
      }
      &.selected {
        background: var(--ctp-surface1);
        color: var(--ctp-mauve);
        transition: background 200ms ease;
      }
    }
  }

  .color-style {
    width: 100%;
    height: 120px;
    display: flex;
    padding: 0 16px;
    align-items: center;
    justify-content: center;
    background: var(--setting-item-background);

    > div {
      width: 90px;
      height: 60px;
      margin: -24px 12px 0px 12px;
      background-position: center;
      background-size: 100%;
      border-radius: 6px;
      transition: all 100ms ease;

      p {
        width: 100%;
        text-align: center;
        font-size: 12.3px;
        margin-top: calc(100% - 20px);
      }
    }

    .latte {
      background-image: url("@/assets/images/catppuccin-latte.webp");
    }

    .frappe {
      background-image: url("@/assets/images/catppuccin-frappe.webp");
    }

    .macchiato {
      background-image: url("@/assets/images/catppuccin-macchiato.webp");
    }

    .mocha {
      background-image: url("@/assets/images/catppuccin-mocha.webp");
    }

    .selected {
      outline: 4px solid var(--ctp-blue);
    }
  }

  .color-style-disabled {
    pointer-events: none;
    * {
      opacity: 0.6;
    }
  }
  .add-account {
    height: 100%;
    :deep(.account-add) {
      padding: 0;
    }
    :deep(.title) {
      color: var(--ctp-lavender);
      font-size: 22px;
      margin-bottom: 8px;
    }
    :deep(.account-add-container),
    :deep(.auth-code) {
      height: 100%;
    }
    :deep(.cancel-login) {
      display: none;
    }
    :deep(.add-microsoft-account-container) {
      flex: 1;
    }
    :deep(.link) {
      margin: 36px 0;
    }
    :deep(.device-code .link) {
      margin-top: 36px;
    }
    .account-view-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      p.title {
        width: 100%;
      }
      .avatar {
        filter: drop-shadow(0 0 8px rgba(0, 0, 0, 0.5));
      }
      p.profile-name {
        margin-top: 16px;
        font-size: 16px;
      }
      p.profile-type {
        font-size: 13px;
        margin-top: 8px;
        &.microsoft {
          color: var(--ctp-green);
        }
        &.yggdrasil {
          color: var(--ctp-yellow);
        }
        &.offline {
          color: var(--ctp-red);
        }
      }
    }
  }
  .profile-card {
    background: var(--ctp-surface0);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    padding: 24px;
    margin-top: 24px;
    width: 320px;
  }
  .java-settings,
  .game-settings {
    :deep(.setting-group),
    :deep(.setting-collapse) {
      width: 100%;
    }
  }
  .import-instances {
    button {
      width: 240px;
      appearance: none;
      background: var(--ctp-lavender);
      color: var(--ctp-text-inverse);
      border: none;
      height: 36px;
      font-size: 12px;
      border-radius: 8px;
      &:hover {
        background-image:
          linear-gradient(#ffffff2f, #ffffff2f),
          linear-gradient(var(--ctp-lavender), var(--ctp-lavender));
      }
      &:active {
        background-image:
          linear-gradient(#ffffff6f, #ffffff6f),
          linear-gradient(var(--ctp-lavender), var(--ctp-lavender));
      }
    }

    button.create-latest-instance {
      &.creating,
      &.error {
        opacity: 0.7;
        pointer-events: none;
      }
      &.error {
        background: var(--ctp-red);
      }
    }
  }
}
</style>
