<template>
  <div class="instances">
    <div class="overview">
      <!-- <card id="xx" title="最新版本" description="Minecraft 1.20.3" icon="grass-block" padding="10,12,10,12" margin="0,0,10,0"
        icon-size="32,32" :icon-background="false" title-font-size="14.2px">
      </card>
      <card id="xxx" title="最新快照" description="Minecraft 1.20.3" icon="command-block" padding="10,12,10,12"
        icon-size="32,32" margin="0,0,10,0" :icon-background="false" title-font-size="14.2px">
      </card> -->
      <card v-for="instance in props.instances" :key="instance.config.name" :id="instance.config.name"
        :title="instance.config.name" :description="generateDescription(instance)" icon="command-block"
        padding="10,12,10,12" icon-size="32,32" margin="0,0,10,0" :icon-background="false" title-font-size="14.2px"
        @click="$emit('select', instance)">
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
  config: {
    name: string,
    runtime: string,
  },
  installed: boolean
}

interface InstanceGroup {
  name: string,
  instances: Instance[]
}

const props = defineProps<{
  // instances: InstanceGroup[], todo: group
  instances: Instance[]
}>()

function generateDescription(instance: Instance): string {
  return instance.config.name
}

</script>
  
<style lang="less" scoped>
div.instances .icon {
  background-position: center;
  background-size: contain;
  background-repeat: none;
}

div.instances .overview {
  overflow: auto;
  padding-right: 5px; // scroll
  max-height: 240px;
}
</style>
  