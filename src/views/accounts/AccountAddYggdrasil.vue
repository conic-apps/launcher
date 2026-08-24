<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="add-yggdrasil-account-container">
    <div v-if="processing" class="processing">
      <div class="loading">
        <BaseLoading :size="40" :strokeWidth="5" :gap="12"></BaseLoading>
      </div>
      <p class="description">请稍后</p>
    </div>
    <div v-else-if="errorMessage" class="error-state">
      <p class="description error">{{ errorMessage }}</p>
      <div class="buttons">
        <BaseButton @click="errorMessage = ''">返回</BaseButton>
      </div>
    </div>
    <template v-else-if="shouldChooseProfile">
      <p class="description">该帐户拥有多个角色，请选择要添加的角色</p>
      <div class="profile-list-wrapper">
        <ScrollView>
          <ul class="profile-list">
            <li
              v-for="profile in availableProfiles"
              :key="profile.id"
              class="profile-item"
              :class="{
                selected: selectedProfileIds.has(profile.id),
                disabled: profileDisabled(profile),
              }"
              @click="toggleProfile(profile.id)">
              <BaseCheckbox
                :model-value="selectedProfileIds.has(profile.id)"
                @click.stop
                @update:model-value="toggleProfile(profile.id)" />
              <AccountAvatar
                :skin="yggdrasilGetSkinUrl(profile)"
                :size="28"
                :uuid="profile.id"
                class="profile-avatar" />
              <p style="display: flex; flex-direction: column">
                <span class="profile-name">{{
                  profile.name + (profileDisabled(profile) ? " (已添加)" : "")
                }}</span>
              </p>
            </li>
          </ul>
        </ScrollView>
      </div>
      <div class="buttons">
        <BaseButton @click="shouldChooseProfile = false"> 返回 </BaseButton>
        <BaseButton
          :disabled="selectedProfileIds.size === 0"
          style="background: var(--ctp-latte-lavender); color: #000; margin-left: 8px"
          @click="addSelectedProfiles">
          添加档案
        </BaseButton>
      </div>
    </template>
    <template v-else>
      <p class="description">
        使用第三方 Yggdrasil 认证服务器登录 Minecraft
        帐户，适用于皮肤站、自建认证服务器等非官方帐户系统。
      </p>
      <div class="form">
        <div class="form-row">
          <div class="form-label">
            <label>认证服务器地址 (API root)</label>
            <p class="field-description">
              {{ serverName ? "认证服务：" + serverName : "" }}
            </p>
          </div>
          <BaseInput v-model="apiRoot" width="300px" />
        </div>
        <div class="form-row">
          <div class="form-label">
            <label>用户名</label>
          </div>
          <BaseInput v-model="username" width="300px" />
        </div>
        <div class="form-row">
          <div class="form-label">
            <label>密码</label>
          </div>
          <BaseInput v-model="password" width="300px" />
        </div>
      </div>
      <div class="buttons">
        <BaseButton @click="dialogStore.accountAdd.visible = false">{{ "取消" }}</BaseButton>
        <BaseButton
          :disabled="!canSubmit"
          style="background: var(--ctp-latte-lavender); color: #000"
          @click="login">
          验证并添加档案
        </BaseButton>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import AccountAvatar from "@/components/AccountAvatar.vue";
import BaseButton from "@/components/BaseButton.vue";
import BaseCheckbox from "@/components/BaseCheckbox.vue";
import BaseInput from "@/components/BaseInput.vue";
import {
  addYggdrasilAccount,
  AuthResponse,
  getYggdrasilServerInfo,
  yggdrasilAuthenticateAccount,
  yggdrasilGetSkinUrl,
  YggdrasilProfile,
} from "@conic/account";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useAccountStore } from "@/store/account";
import BaseLoading from "@/components/BaseLoading.vue";
import ScrollView from "@/components/ScrollView.vue";
import { useDialogStore } from "@/store/dialog";
import { useConfigStore } from "@/store/config";

const emit = defineEmits(["switch-component-manage"]);

const accountStore = useAccountStore();
const dialogStore = useDialogStore();
const configStore = useConfigStore();

const apiRoot = ref("");
const username = ref("");
const password = ref("");
const processing = ref(false);
const errorMessage = ref("");
const serverName = ref("");
let debounceTimer: ReturnType<typeof setTimeout>;

const canSubmit = computed(() => {
  return (
    apiRoot.value.trim() !== "" && username.value.trim() !== "" && password.value.trim() !== ""
  );
});

watch(apiRoot, (value) => {
  clearTimeout(debounceTimer);
  const trimmed = value.trim();
  if (!trimmed) {
    serverName.value = "";
    return;
  }
  debounceTimer = setTimeout(async () => {
    try {
      const info = await getYggdrasilServerInfo(trimmed);
      const name = info.meta["serverName"];
      if (typeof name === "string") {
        serverName.value = name;
      }
    } catch {
      return;
    }
  }, 500);
});

onUnmounted(() => {
  clearTimeout(debounceTimer);
});

const authResponse = ref<AuthResponse | null>(null);
const shouldChooseProfile = ref(false);

const availableProfiles = computed(() => authResponse.value?.availableProfiles ?? []);
const selectedProfileIds = ref(new Set<string>());

function toggleProfile(id: string) {
  const s = new Set(selectedProfileIds.value);
  if (s.has(id)) {
    s.delete(id);
  } else {
    s.add(id);
  }
  selectedProfileIds.value = s;
}

function profileDisabled(profile: YggdrasilProfile) {
  return Object.values(accountStore.yggdrasil).some((account) => {
    return (
      sameApiRoot(account.api_root, apiRoot.value) &&
      account.profile.name === profile.name &&
      account.profile.id === profile.id
    );
  });
}

onMounted(() => {
  apiRoot.value = "";
  username.value = "";
  password.value = "";
});

async function login() {
  processing.value = true;
  errorMessage.value = "";
  try {
    authResponse.value = await yggdrasilAuthenticateAccount(
      apiRoot.value.trim(),
      username.value.trim(),
      password.value,
    );
    if (!authResponse.value.selectedProfile && authResponse.value.availableProfiles.length > 1) {
      processing.value = false;
      shouldChooseProfile.value = true;
      return;
    }
    const profile = authResponse.value.selectedProfile ?? authResponse.value.availableProfiles[0];
    if (!profile) {
      errorMessage.value = "该帐户没有任何游戏角色";
      processing.value = false;
      return;
    }
    await addYggdrasilAccount({
      api_root: apiRoot.value.trim(),
      username: username.value.trim(),
      access_token: authResponse.value.accessToken,
      client_token: authResponse.value.clientToken,
      identifier: crypto.randomUUID(),
      profile,
      textures: {},
      added_at: Date.now(),
    });
    await accountStore.reloadFromFile();
    if (!configStore.current_account) accountStore.selectNextAccount();
    dialogStore.accountAdd.visible = false;
  } catch (e: unknown) {
    console.error(e);
    errorMessage.value = e instanceof Error ? e.message : "登录失败，请检查服务器地址和凭据";
  }
  processing.value = false;
}

async function addSelectedProfiles() {
  if (!authResponse.value || selectedProfileIds.value.size === 0) {
    return;
  }
  processing.value = true;
  try {
    const profiles = availableProfiles.value.filter((p) => selectedProfileIds.value.has(p.id));
    for (const profile of profiles) {
      await addYggdrasilAccount({
        api_root: apiRoot.value.trim(),
        username: username.value.trim(),
        access_token: authResponse.value.accessToken,
        client_token: authResponse.value.clientToken,
        identifier: crypto.randomUUID(),
        profile,
        textures: {},
        added_at: Date.now(),
      });
    }
    await accountStore.reloadFromFile();
    if (!configStore.current_account) accountStore.selectNextAccount();
    dialogStore.accountAdd.visible = false;
  } catch (e: unknown) {
    console.error(e);
    errorMessage.value = e instanceof Error ? e.message : "添加档案失败";
  }
  processing.value = false;
}

function sameApiRoot(a: string, b: string): boolean {
  try {
    const urlA = new URL(a);
    const urlB = new URL(b);

    const normalizePath = (path: string) => path.replace(/\/+$/, "");

    return urlA.host === urlB.host && normalizePath(urlA.pathname) === normalizePath(urlB.pathname);
  } catch {
    return false;
  }
}
</script>

<style lang="less" scoped>
.add-yggdrasil-account-container {
  width: 100%;
  display: flex;
  flex-direction: column;

  div.form {
    display: flex;
    flex-direction: column;
    gap: 8px;

    div.form-row {
      display: flex;
      align-items: center;
      justify-content: space-between;

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
          max-width: 200px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }
      }
    }
  }

  p.description {
    font-size: 14px;
    line-height: 1.3;
    margin-bottom: 8px;
    flex-shrink: 0;
  }

  p.description.error {
    color: var(--ctp-red);
  }

  div.buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: 12px;
    gap: 8px;
  }

  .processing {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 120px;
    flex-direction: column;
    gap: 8px;
    height: 180px;
  }

  .error-state {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 120px;
    gap: 16px;
  }

  .error-state .buttons {
    width: 100px;
  }

  .profile-list-wrapper {
    max-height: 234px;
    height: fit-content;
    display: flex;
    position: relative;
  }

  .profile-list-wrapper :deep(.wrapper.lenis) {
    flex: 1;
    height: unset;
  }

  .profile-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    padding: 2px 4px;
  }

  .profile-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--ctp-surface1);
    transition: background 0.15s ease;

    &.selected {
      background: var(--ctp-surface2);
      outline: 1px solid var(--ctp-latte-lavender);
    }

    &:hover {
      background: var(--ctp-surface2);
    }

    &:active {
      background: var(--ctp-overlay0);
    }

    .profile-name {
      font-size: 14px;
      font-weight: 500;
      margin-bottom: 2px;
    }
    &.disabled {
      pointer-events: none;
      opacity: 0.4;
    }
  }
}
</style>
