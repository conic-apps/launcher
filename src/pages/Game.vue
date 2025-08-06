<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="game-page-main">
    <div class="row-1">
      <div class="side-name">
        <div
          style="display: flex; justify-content: space-between; align-items: center; height: 100%">
          <p style="margin-left: 4px">{{ $t("game.instances") }}</p>
          <button class="side-button" @click="showCreateInstance = true" style="margin-right: 6px">
            <i class="plus" style="font-size: 12px"></i>
          </button>
        </div>
      </div>
      <instance-list @select="setCurrentInstance"></instance-list>
      <CreateInstance
        :show="showCreateInstance"
        @close="showCreateInstance = false"
        @update="update"></CreateInstance>
    </div>
    <div class="row-2">
      <instance-card
        :button-loading="buttonLoading"
        @launch="launch"
        @install="install"
        :error-type="errorType"></instance-card>
      <instance-details style="margin-top: 16px" @update-instance-list="update"></instance-details>
    </div>
  </div>
</template>

<script setup lang="ts">
import InstanceCard from "./game/InstanceCard.vue";
import InstanceDetails from "./game/InstanceDetails.vue";
import InstanceList from "./game/InstanceList.vue";
import { onMounted, ref, watch, type Ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { useConfigStore } from "@/store/config";
import { useInstanceStore } from "@/store/instance";
import CreateInstance from "./dialogs/CreateInstance.vue";
import { Instance, listInstances } from "@conic/instance";
import { install as conicInstall } from "@conic/install";
import { launch as conicLaunch } from "crates/launch";

const config = useConfigStore();
const installing = ref(false);
const buttonLoading = ref(false);
const showCreateInstance = ref(false);
const errorType: Ref<"launch" | "install" | undefined> = ref();
const instanceStore = useInstanceStore();

function update() {
  listInstances("Name").then((res) => {
    instanceStore.instances = res as Instance[];
    let currentInstance = instanceStore.currentInstance;
    let instances = instanceStore.instances;
    let foundCurrentInstance = instances.find((value) => {
      return value.id === currentInstance.id;
    });
    if (foundCurrentInstance) {
      instanceStore.currentInstance = foundCurrentInstance;
    } else {
      if (!config.accessibility.hide_latest_release) {
        setCurrentInstance(instances[0]);
      } else if (!config.accessibility.hide_latest_snapshot) {
        setCurrentInstance(instances[1]);
      } else {
        setCurrentInstance(instances[2]);
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
    let currentInstanceName = instanceStore.currentInstance.config.name;
    if (currentInstanceName !== "Latest Release" && currentInstanceName !== "Latest Snapshot") {
      return;
    }
    let instances = instanceStore.instances;
    if (!config.accessibility.hide_latest_release) {
      setCurrentInstance(instances[0]);
    } else if (!config.accessibility.hide_latest_snapshot) {
      setCurrentInstance(instances[1]);
    } else {
      setCurrentInstance(instances[2]);
    }
  }
});

function setCurrentInstance(instance: Instance) {
  instanceStore.currentInstance = instance;
}

const install = () => {
  installing.value = true;
  instanceStore.installProgress.set(instanceStore.currentInstance.config.name, {
    step: 0,
    completed: 0,
    total: 0,
  });
  conicInstall(config, instanceStore.currentInstance);
};
const launch = () => {
  buttonLoading.value = true;
  conicLaunch(config, instanceStore.currentInstance);
};

listen("install_success", () => {
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
