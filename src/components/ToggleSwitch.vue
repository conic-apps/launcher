<template>
  <div class="input-box">
    <label>
      <input type="checkbox" v-model="checked" />
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
let checked = false
</script>

<style lang="less" scoped>
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
  transition: 0.5s;
  background-color: #fff;
  border-radius: 200px;
  outline: 1px solid #5a5a5ab7;
  transition: all 0.3s cubic-bezier(0, 0.6, 0.32, 0.99);
}

.input-box label span>div {
  width: 13.5px;
  height: 13.5px;
  border-radius: 200px;
  position: absolute;
  background-color: #5a5a5a;
  transition: all 0.3s cubic-bezier(0, 0.6, 0.32, 0.99);
}

.input-box label input:checked~span {
  outline: 1px solid rgba(var(--theme-color), 1);
  background-color: rgba(var(--theme-color), 1);
}

.input-box:hover span {
  background: #ffffffce;
}

.input-box:hover span>div {
  transform: scale(1.1);
}

.input-box:active span>div {
  transform: scale(0.85);
}

.input-box label input:checked~span>div {
  background-color: #fff !important;
  margin-left: calc(100% - 20px);
}

.input-box input {
  display: none;
}
</style>