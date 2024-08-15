<template>
  <div>
    <card-link
      @click="addMicrosoftAccount"
      title="添加微软帐户"
      description="使用微软帐户登录到Minecraft"
      icon="microsoft"
      margin="10,0,10,0"></card-link>
    <card-link
      title="添加离线模式帐户"
      description="指定用户名登录到Minecraft，但不能加入有正版验证的服务器"
      icon="link-slash"
      margin="0,0,10,0"></card-link>
    <ul v-if="!loading">
      <TransitionGroup name="list-item">
        <list-item
          :logo="account.avatar"
          :title="account.name"
          :description="account.id"
          v-for="account in accounts"
          :havelogo="true"
          :key="account.id"
          :logoPixelated="true">
          <icon-button icon="arrow-rotate-right"></icon-button>
          <icon-button icon="circle-info"></icon-button>
          <icon-button icon="trash-can"></icon-button>
        </list-item>
      </TransitionGroup>
    </ul>
    <p class="text-center text-gray text-italic" v-if="loading">正在加载...</p>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue"
import CardLink from "@/components/CardButton.vue"
import IconButton from "@/components/IconButton.vue"

let loading = ref(false)
let accounts: any = []
function addMicrosoftAccount() {}

function addOfflineAccount() {}
</script>

<style lang="less" scoped></style>
