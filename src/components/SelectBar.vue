<template>
    <div class="select-bar">
        <span class="name">{{ name }}</span>
        <selecter :options="options" :default="props.default"></selecter>
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Selecter from './Select.vue';

const props = withDefaults(defineProps<{
    name: string,
    options: string[],
    // config: string,
    default?: number | null
}>(), {
    default: null
})
</script>

<style lang="less" scoped>
.select-bar {
    display: flex;
    margin-bottom: 10px;
    height: 32px;
}
.select-bar>div:last-child {
    margin-left: auto;
    padding-top: 8px;
}
</style>