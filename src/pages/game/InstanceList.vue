<template>
  <div class="instances">
    <div class="overview">
      <list-item
        @click="$emit('select', instance)"
        v-for="(instance, index) in computedInstances"
        :key="index"
        logo="1"
        :title="instanceDisplayName(instance.config.name, index)"
        :click-able="true">
        <template #icon>
          <img
            style="width: 100%; height: 100%; content-visibility: auto"
            src="@/assets/images/minecraft-icon.svg"
            alt="" />
        </template>
        <template #default>
          <div class="tag">
            <img src="@/assets/images/minecraft.webp" width="14px" height="14px" /><span>{{
              instance.config.runtime.minecraft
            }}</span>
          </div>
          <div class="tag" v-if="!!instance.config.runtime.mod_loader_type">
            <img
              src="@/assets/images/quilt.svg"
              width="14px"
              height="14px"
              v-if="instance.config.runtime.mod_loader_type == 'Quilt'" />
            <img
              src="@/assets/images/fabric.webp"
              width="14px"
              height="14px"
              v-if="instance.config.runtime.mod_loader_type == 'Fabric'" />
            <img
              src="@/assets/images/neoforged.png"
              width="14px"
              height="14px"
              v-if="instance.config.runtime.mod_loader_type == 'Neoforge'" />
            <img
              src="@/assets/images/forge.svg"
              width="14px"
              height="14px"
              v-if="instance.config.runtime.mod_loader_type == 'Forge'" />
            <span>{{ instance.config.runtime.mod_loader_version }}</span>
          </div>
        </template>
      </list-item>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useConfigStore } from "@/store/config";
import ListItem from "@/components/ListItem.vue";
import { useI18n } from "vue-i18n";
import { Instance } from "@/types/instance";
const config = useConfigStore();

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
const i18n = useI18n();

function instanceDisplayName(instanceName: string, vueForIndex: number) {
  if (instanceName == "Latest Release" && vueForIndex <= 1) {
    return i18n.t("game.latestRelease");
  } else if (instanceName == "Latest Snapshot" && vueForIndex <= 1) {
    return i18n.t("game.latestSnapshot");
  } else {
    return instanceName;
  }
}
</script>

<style lang="less" scoped>
div.instances {
  height: calc(100% - 21px);
  width: calc(100% + 8px);
  padding-right: 16px;
  overflow-y: auto;

  .icon {
    background-position: center;
    background-size: contain;
    background-repeat: none;
  }

  .overview {
    margin-left: 6px;
    border-radius: var(--list-border-radius);
    overflow: hidden;
    height: fit-content;
    margin-bottom: 10px;
  }
}

div.instance {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  transition: all 50ms cubic-bezier(0, 0, 0.2, 1);
  overflow-y: auto;

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

div.tag {
  display: inline-flex;
  height: 16px;
  align-items: center;
  justify-content: center;
  border: var(--controllers-border);
  border-radius: 100px;
  margin-right: 4px;

  span {
    margin-left: 3px;
  }
}
</style>
