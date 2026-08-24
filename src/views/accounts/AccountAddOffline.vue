<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="add-offline-account-container">
    <p class="description">
      不使用任何认证服务登录帐户。通常情况下，多人游戏中皮肤将不可见，且无法加入启用正版验证的服务器。
    </p>
    <div class="form">
      <div class="form-row">
        <div class="form-label">
          <label>用户名</label>
        </div>
        <BaseInput v-model="username" width="260px" placeholder="用户名" />
      </div>
      <div class="form-row">
        <div class="form-label">
          <label>启用高级设置</label>
        </div>
        <BaseSwitch v-model="advancedMode" />
      </div>
      <div class="form-row" :class="{ disabled: !advancedMode }">
        <div class="form-label">
          <label>UUID</label>
          <p class="field-description" :style="{ color: uuidInvalid ? 'var(--ctp-red)' : '' }">
            {{ uuidInvalid ? "输入的 UUID 无效" : "Minecraft 玩家的唯一标识符" }}
          </p>
        </div>
        <BaseInput
          v-model="uuid"
          width="260px"
          :disabled="!advancedMode"
          placeholder="UUID"
          :error="uuidInvalid" />
      </div>
    </div>
    <div class="buttons">
      <BaseButton @click="dialogStore.accountAdd.visible = false">{{ "取消" }}</BaseButton>
      <BaseButton
        :disabled="username.trim() === '' || uuidInvalid"
        style="background: var(--ctp-latte-lavender); color: #000"
        @click="submitAccount">
        创建离线档案
      </BaseButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import BaseButton from "@/components/BaseButton.vue";
import BaseInput from "@/components/BaseInput.vue";
import BaseSwitch from "@/components/BaseSwitch.vue";
import { useAccountStore } from "@/store/account";
import { useConfigStore } from "@/store/config";
import { useDialogStore } from "@/store/dialog";
import { addOfflineAccount, getUuidFromUsername } from "@conic/account";
import { computed, onMounted, ref, watch } from "vue";

const emit = defineEmits(["switch-component-manage"]);

const accountStore = useAccountStore();
const dialogStore = useDialogStore();
const configStore = useConfigStore();

const username = ref("");
const uuid = ref("");
const advancedMode = ref(false);
const generatedUuid = computed(() => {
  return getUuidFromUsername(username.value);
});

const uuidPattern =
  /^[0-9a-f]{32}$|^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const uuidInvalid = computed(() => {
  return advancedMode.value && uuid.value.trim() !== "" && !uuidPattern.test(uuid.value.trim());
});

onMounted(() => {
  uuid.value = generatedUuid.value;
});

watch(advancedMode, (enabled) => {
  if (enabled) {
    uuid.value = "";
  } else {
    uuid.value = generatedUuid.value;
  }
});

watch(generatedUuid, (value) => {
  if (!advancedMode.value) {
    uuid.value = value;
  }
});

async function submitAccount() {
  if (!username.value.trim() || !uuid.value.trim()) {
    return;
  }
  await addOfflineAccount(username.value.trim(), uuid.value.trim());
  await accountStore.reloadFromFile();
  if (!configStore.current_account) accountStore.selectNextAccount();
  dialogStore.accountAdd.visible = false;
}
</script>

<style lang="less" scoped>
.add-offline-account-container {
  width: 100%;

  p.description {
    font-size: 14px;
    line-height: 1.3;
    margin-bottom: 8px;
  }

  div.form {
    display: flex;
    flex-direction: column;
    gap: 8px;

    div.form-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      transition: opacity 0.2s ease;

      &.disabled {
        opacity: 0.4;
      }

      div.form-label {
        display: flex;
        flex-direction: column;

        label {
          font-size: 13px;
        }

        p.field-description {
          font-size: 12px;
          color: rgba(var(--default-text-color), 0.5);
          margin-top: 2px;
        }
      }
    }
  }

  div.buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: 12px;
    gap: 8px;
  }
}
</style>
