<template>
  <div class="input-box" :style="`width: ${width};`">
    <input @focusin="updateOld" @focusout="checkValue" :type="numberOnly ? 'number' : 'text'" :title="name" :placeholder="placeholder" required
      v-model="model" :style="error ? 'outline: rgb(127,0,0)' : ''" :disabled="disabled" />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { watch } from "vue";

const props = withDefaults(
  defineProps<{
    name?: string;
    placeholder?: string;
    type?: string;
    error?: boolean;
    width?: string;
    numberOnly?: boolean;
    disabled?: boolean;
  }>(),
  {
    type: "text",
    width: "400px",
    numberOnly: false,
    disabled: false
  },
);

const model = defineModel();
let oldValue: number
function updateOld(event: any) {
  if (props.numberOnly) {
    oldValue = model.value as number
    console.log(oldValue)
  }
}
function checkValue(event: any) {
    console.log(event.target.value.trim())
    if (!props.numberOnly) {
    return;
  }
  let value = event.target.value.trim();
    console.log(oldValue)
    if (!/^[1-9]\d*$|^$/.test(value) || value.length == 0) {
    model.value = oldValue;
    event.target.value = oldValue
  }
}
</script>

<style lang="less" scoped>
.input-box {
  border-radius: var(--border-radius-small);
  margin-left: auto;
  overflow: hidden;
  height: 30px;
  flex-shrink: 0;
  padding: 0;
  font-size: 15px;
  transition: all 0.1s ease;
  background-color: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.input-box input {
  border: none;
  background-color: #00000000;
  padding: 0;
  height: 100%;
  width: 100%;
  margin: 0;
  padding: 2px 8px;
  text-align: inherit;
}

.input-box:hover {
  background: rgba(255, 255, 255, 0.08);
}

.input-box:focus-within {
  outline: 1px solid #4493f8;
  background-color: rgba(255, 255, 255, 0.05);
}
</style>
