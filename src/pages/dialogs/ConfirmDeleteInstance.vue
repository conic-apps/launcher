<template>
  <dialog-vue :visible="props.visible" :width="500" :height="300">
    <div class="confirm-delete-instance">
      <p
        style="
          margin-top: -4px;
          margin-bottom: 16px;
          padding-bottom: 16px;
          border-bottom: var(--card-border);
        ">
        Delete Instance
      </p>
      <div
        v-if="!deleting"
        class="dialog-button"
        @click="
          confirmInputText = '';
          $emit('close');
        ">
        <i></i>
      </div>
      <div class="icon">
        <img
          style="width: 100%; height: 100%; content-visibility: auto"
          src="@/assets/images/minecraft-icon.svg"
          alt="" />
      </div>
      <p class="instance-name">
        {{ instance.config.name }}
      </p>
      <div class="instance-info">
        <div>
          <img src="@/assets/images/minecraft.webp" /><span>{{
            instance.config.runtime.minecraft
          }}</span>
        </div>
        <div style="margin-left: 16px">
          <img
            src="@/assets/images/quilt.svg"
            v-if="instance.config.runtime.mod_loader_type == 'Quilt'" />
          <img
            src="@/assets/images/fabric.webp"
            v-if="instance.config.runtime.mod_loader_type == 'Fabric'" />
          <img
            src="@/assets/images/neoforged.png"
            v-if="instance.config.runtime.mod_loader_type == 'Neoforge'" />
          <img
            src="@/assets/images/forge.svg"
            v-if="instance.config.runtime.mod_loader_type == 'Forge'" />
          <span>{{ instance.config.runtime.mod_loader_version }}</span>
        </div>
        <div style="margin-left: 16px">
          <i class="clock"></i>
          <span>114514h</span>
        </div>
      </div>
      <p style="user-select: text; -webkit-user-select: text; cursor: text">
        To confirm, type "{{ instance.config.name }}" in the box below
      </p>
      <TextInputBox
        width="100%"
        style="border: 1px solid rgba(210, 15, 57, 0.8)"
        v-model="confirmInputText">
      </TextInputBox>
      <div class="buttons">
        <button-vue
          text="Cancel"
          style="width: 100%; margin-right: 8px"
          :disabled="deleting"
          @click="
            confirmInputText = '';
            $emit('close');
          "></button-vue>
        <button-vue
          :text="deleting ? 'Deleting...' : 'Delete this instance'"
          style="width: 100%; font-weight: bold"
          @click="confirmDelete"
          :disabled="confirmInputText !== instance.config.name || deleting"
          color="rgb(210, 15, 57)"></button-vue>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import { Instance } from "@/types/instance";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import ButtonVue from "@/components/controllers/Button.vue";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{
  visible: boolean;
  instance: Instance;
}>();

const confirmInputText = ref("");

const emit = defineEmits(["close", "deleted"]);

const deleting = ref(false);
const confirmDelete = () => {
  deleting.value = true;
  invoke("delete_instance", { instanceName: props.instance.config.name }).then(() => {
    deleting.value = false;
    confirmInputText.value = "";
    emit("deleted");
  });
};
</script>

<style lang="less" scoped>
.confirm-delete-instance {
  width: 100%;
  height: 100%;
  padding: 12px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;

  div.icon {
    width: 30px;
    height: 30px;
    opacity: 0.9;
  }

  p.instance-name {
    width: 100%;
    font-size: 22px;
    text-align: center;
    margin-top: 8px;
  }

  .instance-info {
    font-size: 14px;
    display: flex;
    margin-top: 10px;
    width: calc(100% - 32px);
    justify-content: center;

    img {
      width: 14px;
      height: 14px;
      margin-right: 4px;
    }

    i {
      font-family: fa-pro;
      font-style: normal;
      margin-right: 4px;
    }

    > div {
      display: flex;
      align-items: center;

      opacity: 0.8;
    }
  }

  > p {
    font-size: 16px;
    margin: 16px 0 8px 0;
    width: 100%;
  }

  div.buttons {
    margin-top: 8px;
    width: 100%;
    display: flex;
  }
}

.dialog-button {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  position: absolute;
  top: 4px;
  right: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 100ms;
  background: var(--close-btn-background);

  i::before {
    content: "\f00d";
    font-size: 12px;
    margin-top: 1px;
    margin-left: 0.6px;
    font-style: normal;
    font-family: "fa-pro";
    opacity: 0;
    transition: all 70ms ease;
  }

  i {
    transition: all 100ms ease;
  }
}

.button:hover {
  i::before {
    opacity: 1;
  }
}

.button:active {
  i {
    opacity: 0.7;
  }
}
</style>
