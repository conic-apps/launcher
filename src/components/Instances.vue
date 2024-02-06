<template>
  <div class="instances">
    <div class="overview">
      <!-- <card id="xx" title="最新版本" description="Minecraft 1.20.3" icon="grass-block" padding="10,12,10,12" margin="0,0,10,0"
        icon-size="32,32" :icon-background="false" title-font-size="14.2px">
      </card>
      <card id="xxx" title="最新快照" description="Minecraft 1.20.3" icon="command-block" padding="10,12,10,12"
        icon-size="32,32" margin="0,0,10,0" :icon-background="false" title-font-size="14.2px">
      </card> -->
      <!-- <card v-for="instance in props.instances" :key="instance.config.name" :id="instance.config.name"
        :title="instance.config.name" :description="generateDescription(instance)" icon="command-block"
        padding="10,12,10,12" icon-size="32,32" margin="0,0,10,0" :icon-background="false" title-font-size="14.2px"
        @click="$emit('select', instance)">
      </card> -->
      <div class="instance" @click="$emit('select', instance)" v-for="instance in props.instances" :key="instance.config.name">
        <img src="@/assets/images/minecraft-icon.svg">
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
  padding-left: 10px;
  overflow: auto;
  max-height: 240px;
  margin-bottom: 10px;
}

div.instance {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  transition: all 100ms cubic-bezier(0, 0, 0.2, 1);
  
  img {
    width: 24px;
    height: 24px;
    margin-right: 10px;
  }
  .title {
    p {
      margin: 0;
    }
  }
}

div.instance:hover{
  background-color: #3d3d3d;
  box-shadow: 0px 0px 3px #0000004c;
  border-radius: 8px;
  // transition: background-color 0ms cubic-bezier(0, 0, 0.2, 1);
}

div.instance:active {
  transform: scale(0.96);
  opacity: 0.7;
}
</style>
  