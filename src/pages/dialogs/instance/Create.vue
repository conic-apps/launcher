<template>
  <div>
    <div class="info-bar">
      <div class="icon"></div>
      <div class="info" style="display: flex; align-items: center">
        <!-- <h4>
          {{ instanceName }}
          
        </h4> -->
        <text-input-box :error="true" class="instance-name-input" style="width: 300px; margin-left: 10px" name="实例名称"
          :placeholder="defaultInstanceName" v-model="instanceNameValue"></text-input-box>
        <tag v-if="repeated" text="名称不能重复" :color="['255', '129', '120']" text-color="rgba(255,255,255, 0.7)"
          :background="true" :border="true" style="margin-left: 10px">
        </tag>
      </div>
      <vue-button text="创建" :disabled="!select.minecraft || repeated" @click="create"></vue-button>
      <!-- <button style="margin-left: auto;" :class="select.minecraft && !repeated
          ? 'command-button'
          : 'command-button disabled'
        " >
        创建
      </button> -->
    </div>
    <div style="display: flex">
      <div style="width: 100%; margin-right: 4px">
        <card-link margin="0,0,8,0" title="Minecraft"
          :description="select.minecraft ? `已选择 ${select.minecraft}` : `选择 Minecraft 版本`" icon="minecraft"
          @click="showMinecraft = true"></card-link>
        <minecraft-choose :show="showMinecraft" @select="setMinecraft"></minecraft-choose>
        <card-link :class="select.minecraft && !select.fabric && !select.quilt && !noForge && !forgeLoading
          ? ''
          : 'disabled'
          " margin="0,0,0,0" title="Forge" @click="showForge = true" :description="forgeDesc" icon="forge"></card-link>
        <forge-choose :show="showForge" @select="setForge" @no-version="noForge = true" @loaded="forgeLoading = false"
          :minecraft="select.minecraft"></forge-choose>
      </div>
      <div style="width: 100%; margin-left: 4px">
        <card-link :class="select.minecraft && !select.forge && !select.quilt && !noFabric && !fabricLoading
          ? ''
          : 'disabled'
          " @click="showFabric = true" margin="0,0,8,0" title="Fabric" :description="fabricDesc"
          icon="fabric"></card-link>
        <fabric-choose :show="showFabric" @select="setFabric" @no-version="noFabric = true"
          @loaded="fabricLoading = false" :minecraft="select.minecraft"></fabric-choose>
        <card-link @click="showQuilt = true" :class="select.minecraft && !select.forge && !select.fabric && !noQuilt && !quiltLoading
          ? ''
          : 'disabled'
          " margin="0,0,0,0" title="Quilt" :description="quiltDesc" icon="quilt"></card-link>
        <quilt-choose :show="showQuilt" @select="setQuilt" @no-version="noQuilt = true" @loaded="quiltLoading = false"
          :minecraft="select.minecraft"></quilt-choose>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from "vue";
import Expander from "@/components/Expander.vue";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import CardLink from "@/components/CardButton.vue";
import MinecraftChoose from "../MinecraftChoose.vue";
import FabricChoose from "../FabricChoose.vue";
import ForgeChoose from "../ForgeChoose.vue";
import QuiltChoose from "../QuiltChoose.vue";
import { invoke } from "@tauri-apps/api/core";
import VueButton from "@/components/controllers/Button.vue";
import Tag from "@/components/Tag.vue";

const emit = defineEmits(["created"]);
interface Instance {
  config: {
    name: string;
    runtime: string;
  };
  installed: boolean;
}
const props = defineProps<{
  instances: Instance[];
}>();

let showMinecraft = ref(false);
let showForge = ref(false);
let showFabric = ref(false);
let showQuilt = ref(false);

let noFabric = ref(false);
let noForge = ref(false);
let noQuilt = ref(false);

let forgeLoading = ref(false);
let fabricLoading = ref(false);
let quiltLoading = ref(false);

let forgeDesc = computed(() => {
  if (select.forge) {
    return `已选择 ${select.forge}`;
  } else {
    if (select.minecraft) {
      if (select.fabric) {
        return `与 Fabric 不兼容`;
      } else if (select.quilt) {
        return `与 Quilt 不兼容`;
      } else if (noForge.value) {
        return "无可用版本";
      } else if (forgeLoading.value) {
        return "正在加载...";
      } else {
        return "选择 Forge 版本";
      }
    } else {
      return "请先选择 Minecraft 版本";
    }
  }
});

let fabricDesc = computed(() => {
  if (select.fabric) {
    return `已选择 ${select.fabric}`;
  } else {
    if (select.minecraft) {
      if (select.forge) {
        return `与 Forge 不兼容`;
      } else if (select.quilt) {
        return `与 Quilt 不兼容`;
      } else if (noFabric.value) {
        return "无可用版本";
      } else if (fabricLoading.value) {
        return "正在加载...";
      } else {
        return "选择 Fabric 版本";
      }
    } else {
      return "请先选择 Minecraft 版本";
    }
  }
});
let quiltDesc = computed(() => {
  if (select.quilt) {
    return `已选择 ${select.quilt}`;
  } else {
    if (select.minecraft) {
      if (select.fabric) {
        return `与 Fabric 不兼容`;
      } else if (select.forge) {
        return `与 Forge 不兼容`;
      } else if (noQuilt.value) {
        return "无可用版本";
      } else if (quiltLoading.value) {
        return "正在加载...";
      } else {
        return "选择 Quilt 版本";
      }
    } else {
      return "请先选择 Minecraft 版本";
    }
  }
});

let instanceNameValue = ref("");

let select = reactive({
  minecraft: "",
  forge: "",
  fabric: "",
  quilt: "",
  neoforge: "", // TODO: support neoforge
});

let defaultInstanceName = computed(() => {
  return `${select.minecraft ? select.minecraft : "未命名配置"}${select.forge ? "-forge " + select.forge : ""}${select.fabric ? "-fabric " + select.fabric : ""}${select.quilt ? "-quilt " + select.quilt : ""}`;
});

let instanceName = computed(() => {
  if (instanceNameValue.value.trim() === "") {
    return defaultInstanceName.value;
  } else {
    return instanceNameValue.value.trim();
  }
});
//todo: get version list & choose version
function setMinecraft(versionId: string) {
  let old = select.minecraft;
  showMinecraft.value = false;
  if (old != versionId) {
    select.minecraft = versionId;
    select.fabric = "";
    select.forge = "";
    select.quilt = "";
    noFabric.value = false;
    noForge.value = false;
    noQuilt.value = false;
    fabricLoading.value = true;
    forgeLoading.value = true;
    quiltLoading.value = true;
  }
}
function setForge(versionId: string) {
  select.forge = versionId;
  showForge.value = false;
}
function setFabric(versionId: string) {
  select.fabric = versionId;
  showFabric.value = false;
}
function setQuilt(versionId: string) {
  select.quilt = versionId;
  showQuilt.value = false;
}
function create() {
  if (select.minecraft == "" || instanceName.value.trim() == "" || repeated.value) {
    return;
  }
  // TODO: rewrite this ui, choose mod loader type,then choose version
  let mod_loader_type: string | null
  let mod_loader_version: string | null
  if (select.fabric) {
    mod_loader_type = "Fabric"
    mod_loader_version = select.fabric
  } else if (select.quilt) {
    mod_loader_type = "Quilt"
    mod_loader_version = select.quilt
  } else if (select.forge) {
    mod_loader_type = "Forge"
    mod_loader_version = select.forge
  } else if (select.neoforge) {
    mod_loader_type = "Neoforge"
    mod_loader_version = select.neoforge
  } else {
    mod_loader_type = null
    mod_loader_version = null
  }
  invoke("create_instance", {
    instanceName: instanceName.value,
    config: {
      name: instanceName.value,
      runtime: {
        minecraft: select.minecraft,
        mod_loader_type,
        mod_loader_version,
      },
    },
  })
    .then((res: any) => {
      emit("created");
      setTimeout(() => {
        select.minecraft = "";
        select.forge = "";
        select.fabric = "";
        select.quilt = "";
        instanceNameValue.value = "";
      }, 500);
    })
    .catch((err: any) => {
      console.log(err);
    });
}

let repeated = ref(false);

watch(instanceName, (newValue) => {
  invoke("check_repeated_instance_name", {
    instanceName: newValue,
  }).then((res: any) => {
    console.log(res);
    repeated.value = res;
  });
});
</script>

<style lang="less" scoped>
.info-bar {
  display: flex;
  margin-bottom: 12px;
  padding: 20px 24px;
  margin-top: 10px;
  align-items: center;
  background: rgba(255, 255, 255, 0.04);
  border-radius: 10px;
}

.info-bar .icon {
  background-image: url(@/assets/images/minecraft-icon.svg);
  background-size: cover;
  width: 36px;
  height: 36px;
  margin-right: 8px;
}

.info h4 {
  font-weight: 500;
  font-size: 17px;
  line-height: 1;
  display: flex;
}

.info h4>div {
  color: rgb(172, 0, 0);
  margin-left: 10px;
}

.info p {
  opacity: 0.6;
  text-align: left;
  padding-left: 2px;
}

.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.instance-name-input input {
  padding: 0 !important;
  outline: none !important;
}
</style>
