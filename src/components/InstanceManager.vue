<template>
  <div class="instance-manager">
    <div class="overview">
      <card id="xx" title="最新版本" description="Minecraft 1.20.3" icon="grass-block" padding="10,12,10,12" margin="0,0,10,0"
        icon-size="32,32" :icon-background="false" title-font-size="14.2px">
      </card>
      <card id="xxx" title="最新快照" description="Minecraft 1.20.3" icon="command-block" padding="10,12,10,12"
        icon-size="32,32" margin="0,0,10,0" :icon-background="false" title-font-size="14.2px">
      </card>
      <card v-for="instance in instances" :key="instance.id" :id="instance.id" :title="instance.name"
        description="Minecraft 1.20.3" icon="command-block" padding="10,12,10,12" icon-size="32,32" margin="0,0,10,0"
        :icon-background="false" title-font-size="14.2px">
      </card>
    </div>
    <!-- <card id="xx" cla/ss="overview" :title="instance.name" :description="generateDescription(instance)" padding="10,12,10,12" -->
    <!-- :icon="instance.icon" v-for="(instance, index) in instances" :key="index"> -->
    <!-- </card> -->
    <!-- id is random sha1-->
  </div>
</template>
  
<script setup lang="ts">
import { ref, type Ref } from 'vue';
import Card from './Card.vue';
interface Instance {
  name: string;
  icon: string;
  id: string;
}

const instances: Ref<Instance[]> = ref([
  {
    name: " 1.18.2 加小浣熊",
    icon: "grass-block",
    id: "xx",
  },
  {
    name: "1.20",
    icon: "grass-block",
    id: "xx",
  },
  {
    name: "1.20",
    icon: "grass-block",
    id: "xx",
  },
  {
    name: "1.20",
    icon: "grass-block",
    id: "xx",
  },
  {
    name: "1.20",
    icon: "grass-block",
    id: "xx",
  },
])

function generateDescription(instance: Instance): string {
  return instance.name
}

</script>
  
<style lang="less" scoped>
div.instance-manager .icon {
  background-position: center;
  background-size: contain;
  background-repeat: none;
}

div.instance-manager .overview {
  overflow: auto;
  padding-right: 5px; // scroll
  max-height: 240px;
}
</style>
  