<template>
  <div class="input">
    <span class="name">{{ name }}</span>
    <div class="input-box">
      <label>
        <input type="checkbox" @input="updateData" v-model="checked" />
        <span>
          <div></div>
        </span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import { load, update } from '@/utils/ConfigLoader'
import { ref, type Ref } from 'vue'

const props = defineProps<{
  name: string,
  // config: string,
}>()

let checked: Ref<boolean> = ref(false)
function updateData(): void {
  // update(props.config, !checked.value)
}
</script>

<style scoped lang="less">
div.input {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  // position: relative;
  height: 32px;
  pointer-events: none;
}

span.name {
  font-size: 14px;
  /* height: 1.4em; */
  min-width: 10em;
  margin-right: 14px;
  transition: color 0.2s ease;
}

.input-box {
  width: 40px;
  height: 20px;
  display: flex;
  justify-content: space-between;
  margin: 0 6px;
  pointer-events: all;
}

.input-box label {
  position: relative;
  /* cursor: pointer; */
  display: flex;
  align-items: center;
}

.input-box label span {
  width: 39px;
  height: 20px;
  display: flex;
  align-items: center;
  padding: 0 3px;
  background-color: #f6f6f6;
  box-shadow: 0 0 0 1px #868585;
  border-radius: 200px;
  transition: all 0.3s cubic-bezier(0, 0.6, 0.32, 0.99);
}

.input-box label span>div {
  width: 13.5px;
  height: 13.5px;
  border-radius: 200px;
  // position: absolute;
  background-color: #636363;
  transition: all 0.3s cubic-bezier(0, 0.6, 0.32, 0.99);
}

.input-box label input:checked~span {
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 1);
  background-color: rgba(var(--theme-color), 1);
}

.input-box:hover span>div {
  transform: scale(1.1);
}

.input-box:active span>div {
  transform: scale(0.85);
}

.input-box label input:checked~span>div {
  background-color: #fff !important;
  margin-left: calc(100% - 14px);
}

.input-box input {
  display: none;
}
</style>