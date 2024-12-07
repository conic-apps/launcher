<template>
  <div class="game-page-main">
    <div class="row-1">
      <div class="side-name">
        <div
          style="display: flex; justify-content: space-between; align-items: center; height: 100%">
          <p style="margin-left: 4px">{{ $t("game.instances") }}</p>
          <button
            class="side-button"
            @click="show.instanceManager = true"
            style="margin-right: 6px">
            <i class="chevron-right" style="font-size: 12px"></i>
          </button>
        </div>
      </div>
      <instance-list :instances="instances" @select="setCurrentInstance"></instance-list>
      <instance-manager
        :show="show.instanceManager"
        @close="show.instanceManager = false"
        :instances="instances"
        @update="update"></instance-manager>
    </div>
    <div class="row-2">
      <install-progress
        :installing="installing"
        :instance-name="currentInstance.config.name"
        :mod-loader-type="currentInstance.config.runtime.mod_loader_type"
        :mod-loader-version="currentInstance.config.runtime.mod_loader_version"></install-progress>
      <instance-card
        :minecraft-version="currentInstance.config.runtime.minecraft"
        :mod-loader-type="currentInstance.config.runtime.mod_loader_type"
        :mod-loader-version="currentInstance.config.runtime.mod_loader_version"
        :instance-name="currentInstance.config.name"
        :installed="true"
        :game-button-type="gameButtonType"
        :button-loading="buttonLoading"
        @game-button-click="gameButtonClick"
        :error-type="errorType"></instance-card>
      <assets-manager
        :instance="currentInstance"
        style="margin-top: 16px"
        @update-instance-list="update"></assets-manager>
    </div>
  </div>
</template>

<script setup lang="ts">
import InstanceCard from "./game/InstanceCard.vue";
import AssetsManager from "./game/AssetsManager.vue";
import InstallProgress from "./dialogs/InstallProgress.vue";
import InstanceList from "./game/InstanceList.vue";
import InstanceManager from "@/pages/dialogs/InstanceManager.vue";
import { onMounted, ref, watch, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useConfigStore } from "@/store/config";
import { Instance } from "@/types/instance";

const config = useConfigStore();

const installing = ref(false);

const buttonLoading = ref(false);

const currentInstance = ref<Instance>({
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
const show = ref({
  instanceManager: false,
});
const instances: Ref<Instance[]> = ref([]);
const gameButtonType: Ref<"install" | "launch" | "error"> = ref("install");
const errorType: Ref<"launch" | "install" | undefined> = ref();

function update() {
  invoke("scan_instances_folder").then((res) => {
    instances.value = [];
    let userInstances: Instance[] = [];
    (res as Instance[]).forEach((v) => {
      if (v.config.name === "Latest Release") {
        instances.value[0] = v;
      } else if (v.config.name === "Latest Snapshot") {
        instances.value[1] = v;
      } else {
        userInstances.push(v);
      }
    });
    instances.value.push(...userInstances);
    if (
      !instances.value.find((value) => {
        return value.config.name === currentInstance.value.config.name;
      })
    ) {
      if (!config.accessibility.hide_latest_release) {
        setCurrentInstance(instances.value[0]);
      } else if (!config.accessibility.hide_latest_snapshot) {
        setCurrentInstance(instances.value[1]);
      } else {
        setCurrentInstance(instances.value[2]);
      }
    }
  });
}

onMounted(() => {
  update();
});

let hide_latest_release = config.accessibility.hide_latest_release;
let hide_latest_snapshot = config.accessibility.hide_latest_snapshot;

watch(config, (value) => {
  if (
    value.accessibility.hide_latest_release !== hide_latest_release ||
    value.accessibility.hide_latest_snapshot !== hide_latest_snapshot
  ) {
    hide_latest_release = value.accessibility.hide_latest_release;
    hide_latest_snapshot = value.accessibility.hide_latest_snapshot;
    let currentInstanceName = currentInstance.value.config.name;
    if (currentInstanceName !== "Latest Release" && currentInstanceName !== "Latest Snapshot") {
      return;
    }
    if (!config.accessibility.hide_latest_release) {
      setCurrentInstance(instances.value[0]);
    } else if (!config.accessibility.hide_latest_snapshot) {
      setCurrentInstance(instances.value[1]);
    } else {
      setCurrentInstance(instances.value[2]);
    }
  }
});

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
  }, 1000);
});
</script>

<style lang="less" scoped>
.game-page-main {
  width: 100%;
  height: 100%;
  display: flex;
}

.row-1 {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.row-1 {
  width: 328px;
  padding: 24px 24px;
  flex-shrink: 0;
}

.row-2 {
  width: 100%;
  overflow-y: auto;
  padding: 24px 24px 24px 0;
}

.side-name {
  width: 100%;
  height: 32px;
  font-size: 14.5px;
  margin-bottom: 8px;
}

.side-button {
  background: rgba(255, 255, 255, 0.08);
  border: none;
  border-radius: 4px;
  height: 20px;
  width: 20px;
  font-size: 10px;
}

.side-button:hover {
  background: rgba(255, 255, 255, 0.12);
}

.side-button:active {
  background: rgba(255, 255, 255, 0.08);
}

.side-button i::before {
  transform: scale(0.7);
}
</style>
