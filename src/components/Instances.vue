<template>
  <div class="instances">
    <div class="overview">
      <div class="instance" @click="$emit('select', instance)" v-for="instance in computedInstances"
        :key="instance.config.name">
        <img src="@/assets/images/minecraft-icon.svg" />
        <div class="title">
          <p>{{ instance.config.name }}</p>
        </div>
      </div>
    </div>
    <!-- <card id="xx" cla/ss="overview" :title="instance.name" :description="generateDescription(instance)" padding="10,12,10,12" -->
    <!-- :icon="instance.icon" v-for="(instance, index) in instances" :key="index"> -->
    <!-- </card> -->
    <!-- id is random sha1-->
  </div>
</template>

<script setup lang="ts">
import { computed, ref, type Ref } from "vue";
import Card from "./Card.vue";

export interface Instance {
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

interface InstanceGroup {
  name: string;
  instances: Instance[];
}

const props = defineProps<{
  // instances: InstanceGroup[], todo: group
  instances: Instance[];
}>();

const computedInstances = computed(() => {
  let systemInstances = props.instances.filter((value) => {
    return value.config.name == "Latest Release" || value.config.name == "Latest Snapshot";
  });
  let userInstances = props.instances.filter((value) => {
    return value.config.name != "Latest Release" && value.config.name != "Latest Snapshot";
  });
  systemInstances.push(...userInstances);
  return systemInstances;
});
function generateDescription(instance: Instance): string {
  return instance.config.name;
}
</script>

<style lang="less" scoped>
div.instances .icon {
  background-position: center;
  background-size: contain;
  background-repeat: none;
}

div.instances .overview {
  padding-left: 6px;
  overflow: auto;
  max-height: 240px;
  margin-bottom: 10px;
}

div.instance {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  transition: all 50ms cubic-bezier(0, 0, 0.2, 1);

  img {
    width: 24px;
    height: 24px;
    margin-right: 10px;
    fill: aqua
  }

  .title {
    p {
      margin: 0;
    }
  }
}

div.instance:hover {
  background-color: #ffffff1c;
  box-shadow: 0px 0px 3px #0000004c;
  border-radius: 8px;
  // transition: background-color 0ms cubic-bezier(0, 0, 0.2, 1);
}

div.instance:active {
  transform: scale(0.96);
  opacity: 0.7;
}
</style>
