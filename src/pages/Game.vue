<template>
  <keep-alive>
    <div class="game-page-main">
      <div class="row-1">
        <install-progress
          :installing="installing"
          :instance-name="currentInstance.config.name"
          :mod-loader-type="currentInstance.config.runtime.mod_loader_type"
          :mod-loader-version="
            currentInstance.config.runtime.mod_loader_version
          "></install-progress>
        <instance-info
          :minecraft-version="currentInstance.config.runtime.minecraft"
          :mod-loader-type="currentInstance.config.runtime.mod_loader_type"
          :mod-loader-version="currentInstance.config.runtime.mod_loader_version"
          :instance-name="currentInstance.config.name"
          :installed="true"
          :game-button-type="gameButtonType"
          :button-loading="buttonLoading"
          @game-button-click="gameButtonClick"
          :error-type="errorType"></instance-info>
        <assets-manager :instance="currentInstance" style="margin-top: 20px"></assets-manager>
      </div>
      <div class="row-2">
        <div class="group-name">
          <div
            style="
              display: flex;
              justify-content: space-between;
              align-items: center;
              height: 100%;
            ">
            <p style="margin-left: 4px">{{ $t("game.accounts") }}</p>
            <button class="group-button" style="margin-right: 6px">
              <i class="chevron-right" style="font-size: 12px"></i>
            </button>
          </div>
        </div>
        <accounts></accounts>
        <div class="group-name">
          <div
            style="
              display: flex;
              justify-content: space-between;
              align-items: center;
              height: 100%;
            ">
            <p style="margin-left: 4px">{{ $t("game.instances") }}</p>
            <button
              class="group-button"
              @click="show.instanceManager = true"
              style="margin-right: 6px">
              <i class="chevron-right" style="font-size: 12px"></i>
            </button>
          </div>
        </div>
        <Instances :instances="instances" @select="setCurrentInstance"></Instances>
        <instance-manager
          :show="show.instanceManager"
          @close="show.instanceManager = false"
          :instances="instances"
          @update="update"></instance-manager>
      </div>
      <LogViewer
        :instance-name="currentInstance.config.name"
        :visible="logViewerOpen"
        @close="logViewerOpen = false">
      </LogViewer>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import InstanceInfo from "@/components/InstanceInfo.vue";
import AssetsManager from "@/components/AssetsManager.vue";
import InstallProgress from "./dialogs/InstallProgress.vue";
import Accounts from "@/components/Accounts.vue";
import Instances from "@/components/Instances.vue";
import InstanceManager from "@/pages/dialogs/InstanceManager.vue";
import LogViewer from "./dialogs/LogViewer.vue";
import { ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useConfigStore } from "@/config";

const config = useConfigStore();

const installing = ref(false);

const buttonLoading = ref(false);
const logViewerOpen = ref(false);

interface Instance {
  config: {
    name: string;
    runtime: {
      minecraft: string;
      mod_loader_type: "Fabric" | "Quilt" | "Forge" | "Neoforge" | undefined;
      mod_loader_version: string | undefined;
    };
  };
  installed: boolean;
}

let currentInstance = ref<Instance>({
  config: {
    name: "",
    runtime: {
      minecraft: "",
      mod_loader_type: undefined,
      mod_loader_version: undefined,
    },
  },
  installed: false,
});
let show = ref({
  instanceManager: false,
});
let instances: Ref<Instance[]> = ref([]);
let gameButtonType: Ref<"install" | "launch" | "error"> = ref("install");
let errorType: Ref<"launch" | "install" | undefined> = ref();

function update() {
  invoke("scan_instances_folder").then((res) => {
    instances.value = res as Instance[];
  });
}
invoke("scan_instances_folder").then((res) => {
  instances.value = res as Instance[];
  instances.value.map((instance) => {
    if (instance.config.name == "Latest Release") {
      setCurrentInstance(instance);
    }
  });
});

let modIsLoading = ref(false);
let resourcepacksIsLoading = ref(false);
let shaderpackIsLoading = ref(false);
let savesIsLoading = ref(false);
let mods = ref([]);
let resourcepacks = ref([]);
let shaderpacks = ref([]);
let saves = ref([]);

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
}

function setCurrentInstance(instance: Instance) {
  currentInstance.value = instance;
  gameButtonType.value = instance.installed ? "launch" : "install";
  invoke("set_current_instance", {
    instanceName: instance.config.name,
  });
}

function gameButtonClick() {
  if (gameButtonType.value === "launch") {
    buttonLoading.value = true;
    if (config.accessibility.open_log_viewer) {
      logViewerOpen.value = true;
    }
    invoke("launch", {
      instanceName: currentInstance.value.config.name,
    });
  } else if (gameButtonType.value === "install") {
    installing.value = true;
    invoke("install");
  }
}

listen("install_success", () => {
  gameButtonType.value = "launch";
  setTimeout(() => {
    installing.value = false;
  }, 1500);
  update();
});

listen("launch_success", () => {
  setTimeout(() => {
    buttonLoading.value = false;
  }, 10000);
});
</script>

<style lang="less" scoped>
.game-page-main {
  width: 100%;
  height: 100%;
  display: flex;
}

.row-1,
.row-2 {
  height: 100%;
  display: flex;
  flex-direction: column;
  // border: 1px solid #fff;
}

.row-1 {
  padding: 24px 24px;
  width: 100%;
}

.row-2 {
  width: 328px;
  padding: 24px 24px 24px 0;
  flex-shrink: 0;
}

// todo: move to main.css
.group-name {
  width: 100%;
  height: 32px;
  font-size: 14.5px;
  margin-bottom: 8px;
}

.group-button {
  background: rgba(255, 255, 255, 0.08);
  border: none;
  border-radius: 4px;
  height: 20px;
  width: 20px;
  font-size: 10px;
}

.group-button:hover {
  background: rgba(255, 255, 255, 0.12);
}

.group-button:active {
  background: rgba(255, 255, 255, 0.08);
}

.group-button i::before {
  transform: scale(0.7);
}
</style>
