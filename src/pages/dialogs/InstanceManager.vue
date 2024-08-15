<template>
  <dialog-vue :visible="props.show" width="860" height="520">
    <div class="instance-manager">
      <div style="width: 100%; height: 100%">
        <div class="title" style="position: relative">
          <div style="display: flex; align-items: center">
            <div class="icon">
              <i class="boxes-stacked" v-if="currentComponent == pages.view"></i>
              <i
                class="arrow-left back"
                v-if="currentComponent == pages.create"
                @click="
                  transitionName = 'slide-right'
                  currentComponent = pages.view
                "></i>
            </div>
            <div>
              <h4 v-if="currentComponent == pages.view">管理游戏档案</h4>
              <p v-if="currentComponent == pages.view">创建、删除或修改你的游戏档案</p>
              <h4 v-if="currentComponent == pages.create">创建游戏档案</h4>
              <p v-if="currentComponent == pages.create">选择好合适的选项后，点击“创建”按钮</p>
            </div>
          </div>
          <div
            class="button"
            style="position: absolute; right: 0"
            @click="
              currentComponent = pages.view
              $emit('close')
            ">
            <i></i>
          </div>
        </div>

        <div class="content">
          <Transition :name="transitionName" mode="out-in">
            <component
              :instances="props.instances"
              :is="currentComponent"
              @create="createInstance"
              @created="instanceCreated"></component>
          </Transition>
          <!-- <div class="group" v-for="group in props.instances" :key="group.name">
          <div class="instance" v-for="instance in group.instances" :key="instance.name">
            <p>{{ instance.name }}</p>
          </div>
        </div> todo: group -->
        </div>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue"
import { markRaw, reactive, ref, shallowRef } from "vue"
import View from "@/pages/dialogs/instance/View.vue"
import Create from "@/pages/dialogs/instance/Create.vue"

let emit = defineEmits(["close", "update"])
let transitionName = ref("slide-left")
interface Instance {
  config: {
    name: string
    runtime: string
  }
  installed: boolean
}

interface InstanceGroup {
  name: string
  instances: Instance[]
}

const props = defineProps<{
  show: boolean
  // instances: InstanceGroup[], todo: group
  instances: Instance[]
}>()

const pages: any = reactive({
  view: markRaw(View),
  create: markRaw(Create),
})

const currentComponent = shallowRef(pages.view)

function createInstance() {
  transitionName.value = "slide-left"
  currentComponent.value = pages.create
}

function instanceCreated() {
  transitionName.value = "slide-right"
  emit("update")
  currentComponent.value = pages.view
}
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
  display: flex;
  align-items: center;
  justify-content: center;
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
  font-family: "fa-pro";
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
  content: "\f00d";
  font-size: 12px;
  margin-top: 1px;
  margin-left: 0.6px;
  font-style: normal;
  font-family: "fa-pro";
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

.instance-manager {
  overflow: hidden;
}

.instance-manager .title .icon .back {
  border-radius: 1000px;
  width: 40px;
  height: 40px;
  border: 1px solid rgba(255, 255, 255, 0.38);
}

.instance-manager .title .icon .back:active {
  opacity: 0.7;
}

.instance-manager .title .icon i.back::before {
  font-size: 20px;
}
</style>
