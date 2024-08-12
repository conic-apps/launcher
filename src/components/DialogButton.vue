<template>
    <div class="dialog-button">
        <i :class="icon"></i>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
defineProps<{
    icon: string
}>()
</script>

<style lang="less" scoped>
.dialog-button {
    width: 26px;
    height: 26px;
    border: 1px solid #585858a4;
    border-radius: 10000px;
    display: flex;
    justify-content: center;
    align-items: center;
    transition: all 200ms ease;
}

.dialog-button:hover {
    box-shadow: 0 0 4px #00000025;
    transform: scale(1.06);
}

.dialog-button:active {
    transform: scale(0.95);
}

.dialog-button i {
    font-family: 'fa-pro';
    font-style: normal;
    font-size: calc(16px - var(--));
    display: flex;
    justify-content: center;
    align-items: center;
}</style>