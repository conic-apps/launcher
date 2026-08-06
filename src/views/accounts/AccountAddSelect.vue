<template>
	<div class="selection">
	        <button
	          class="microsoft item"
	          :class="{ active: authServiceType === 'microsoft' }"
	          @click="selectAuthService('microsoft')">
	          <AppIcon name="logo-microsoft" :size="15"></AppIcon>
	          <p>微软</p>
	        </button>
	        <button
	          class="yggdrasil item"
	          :class="{ active: authServiceType === 'yggdrasil' }"
	          @click="selectAuthService('yggdrasil')">
	          <AppIcon name="extension-puzzle" :size="15"></AppIcon>
	          <p>外置</p>
	        </button>
	        <button
	          class="offline item"
	          :class="{ active: authServiceType === 'offline' }"
	          @click="selectAuthService('offline')">
	          <AppIcon name="cloud-offline-outline" :size="15"></AppIcon>
	          <p>离线</p>
	        </button>
	      </div>
</template>

<script setup lang="ts">
defineProps<{
	selectAuthService: (service: "microsoft" | "yggdrasil" | "offline") => void;
}>()
const authServiceType = defineModel<"microsoft" | "offline" | "yggdrasil">('authServiceType', { required: true })
</script>
