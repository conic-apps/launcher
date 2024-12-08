<template>
  <div class="input-box">
    <label>
      <input type="checkbox" v-model="model" />
      <span>
        <div></div>
      </span>
    </label>
  </div>
</template>

<script setup lang="ts">
// const props = defineProps<{
//   checked: boolean
// }>()
const model = defineModel();
</script>

<style lang="less" scoped>
.input-box {
  margin: 0 6px;

  label {
    /* cursor: pointer; */
    display: flex;
    align-items: center;
  }
}

.input-box label span {
  width: 39px;
  height: 20px;
  display: flex;
  align-items: center;
  padding: 0 3px;
  transition: 0.5s;
  background: var(--toggle-switch-background);
  border-radius: 200px;
  outline: var(--toggle-switch-outline);
  transition: all 0.3s cubic-bezier(0, 0.6, 0.32, 0.99);

  div {
    width: 13.5px;
    height: 13.5px;
    border-radius: 200px;
    background: var(--toggle-switch-button);
    transition: all 0.3s cubic-bezier(0, 0.6, 0.32, 0.99);
  }
}

.input-box label input:checked ~ span {
  outline: 1px solid var(--toggle-switch-active);
  background: var(--toggle-switch-active);
}

.input-box:hover span {
  background: rgba(255, 255, 255, 0.08);
}

.input-box:hover span > div {
  transform: scale(1.1);
}

.input-box:active span > div {
  transform: scale(0.85);
}

.input-box label input:checked ~ span > div {
  background-color: #fff;
  margin-left: 19px;
}

.input-box input {
  display: none;
}
</style>
