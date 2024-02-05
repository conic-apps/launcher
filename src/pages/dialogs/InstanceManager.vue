<template>
  <dialog-vue :visible="props.show" width="860" height="520">
    <div class="instance-manager">
      <div class="title" style="position: relative;">
        <div style="display: flex; align-items: center;">
          <div class="icon">
            <i class="boxes-stacked"></i>
          </div>
          <div>
            <h4>管理游戏档案</h4>
            <p>创建、删除或修改你的游戏档案</p>
          </div>
        </div>
        <div class="button" style="position: absolute; right: 0;" @click="$emit('close')"><i></i></div>
      </div>

      <div class="content">
        <!-- <div class="group" v-for="group in props.instances" :key="group.name">
          <div class="instance" v-for="instance in group.instances" :key="instance.name">
            <p>{{ instance.name }}</p>
          </div>
        </div> todo: group -->

        <div class="instance" v-for="instance in props.instances" :key="instance.config.name">
          <img src="@/assets/images/Grass_Block.webp">
          <p>{{ instance.config.name }}</p>
        </div>
        <div class="instance">
          <i class="plus"></i>
        </div>
      </div>
    </div>

  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import DialogButton from "@/components/DialogButton.vue";
import { tauri } from "@tauri-apps/api";
import { watch } from "vue";

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
  show: boolean,
  // instances: InstanceGroup[], todo: group
  instances: Instance[]
}>()

watch(props, (newValue) => {
  console.log(newValue.instances)
})

</script>

<style lang="less" scoped>
.instance-manager {
  width: 100%;
  height: 100%;
}

.instance-manager .title {
  border-bottom: 1px solid #ffffff18;
  width: 100%;
  height: 80px;
  display: flex;
  justify-content: space-between;
}

.instance-manager .title .icon {
  width: 80px;
  height: 80px;
}

.instance-manager .title i {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.instance-manager .title .icon i::before {
  font-size: 36px;
  font-style: normal;
  font-family: 'fa-pro';
}

.instance-manager h4 {
  font-size: 22px;
  font-weight: normal;
}

.instance-manager p {
  font-size: 14px;
  margin-top: 4px;
  opacity: 0.7;
  font-weight: normal;
}

.instance-manager .button {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  margin-left: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  background: #ffffff40;
}

.instance-manager .button i::before {
  content: '\f00d';
  font-size: 12px;
  margin-top: 1px;
  margin-left: 0.6px;
  font-style: normal;
  font-family: 'fa-pro';
  opacity: 0;
  transition: all 70ms ease;
}

.instance-manager .button i {
  transition: all 100ms ease;
}

.instance-manager .button:hover i::before {
  opacity: 1;
}

.instance-manager .button:active i {
  opacity: 0.7;
}


.instance-manager .commands {
  width: fit-content;
  height: 60px;
  display: flex;
  height: 100%;
  align-items: center;
  padding-right: 20px;
}

.instance-manager .commands>div {
  font-size: 14px;
  border: 1px solid #ffffff39;
  padding: 8px 10px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;

}

.instance-manager .commands i::before {
  font-size: 16px;
  font-style: normal;
  font-family: 'fa-pro';
  line-height: 1;
  text-align: center
}

.instance-manager .content {
  display: flex;
  padding-top: 10px;
}

.instance-manager .content .instance {
  display: flex;
  width: 100px;
  height: 100px;
  align-items: center;
  flex-direction: column;
  justify-content: center;
  margin: 4px;
  border-radius: 10px;
}

.instance-manager .content .instance:hover {
  background: #ffffff0c;

}

.instance-manager .content .instance img {
  width: 48px;
  height: 48px;
}

.instance-manager .content .instance i {
  font-family: 'fa-pro';
  font-style: normal;
  font-size: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px dotted #ffffffba;
  width: 40px;
  height: 40px;
  opacity: 0.6;
  border-radius: 10px;
}

.instance-manager .content .instance p {
  text-align: center;
  margin-top:8px;
  font-size: 13px;
}
</style>
