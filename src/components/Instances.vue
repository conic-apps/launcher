<template>
  <div class="instances">
    <div class="overview">
      <div
        class="instance"
        @click="$emit('select', instance)"
        v-for="(instance, index) in computedInstances"
        :key="instance.config.name">
        <img src="@/assets/images/minecraft-icon.svg" />
        <div class="title">
          <p v-if="instance.config.name == 'Latest Release' && index <= 1">
            {{ $t("game.latestRelease") }}
          </p>
          <p v-else-if="instance.config.name == 'Latest Snapshot' && index <= 1">
            {{ $t("game.latestSnapshot") }}
          </p>
          <p v-else>
            {{ instance.config.name }}
          </p>
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
import { useConfigStore } from "@/config";
const config = useConfigStore();

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
  let result = props.instances.filter((value) => {
    return value.config.name == "Latest Release" || value.config.name == "Latest Snapshot";
  });
  let userInstances = props.instances.filter((value) => {
    return value.config.name != "Latest Release" && value.config.name != "Latest Snapshot";
  });
  result.push(...userInstances);
  if (config.accessibility.hide_latest_release) {
    result = result.filter((value) => {
      return value.config.name != "Latest Release";
    });
  }
  if (config.accessibility.hide_latest_snapshot) {
    result = result.filter((value) => {
      return value.config.name != "Latest Snapshot";
    });
  }
  return result;
});
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
    fill: aqua;
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
