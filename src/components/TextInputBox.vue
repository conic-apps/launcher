<template>
  <div class="input-data mini">
    <input type="text" :title="name" :placeholder="placeholder" required :value="value" />
    <div class="underline"></div>
  </div>
</template>

<script setup lang="ts">
withDefaults(defineProps<{
  name?: string,
  placeholder?: string,
  value?: string,
  type?: string,
}>(), {
  type: "text"
})

</script>

<style lang="less" scoped>
.input-data {
  border-radius: var(--border-radius-small);
  margin-left: auto;
  width: 400px;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(var(--theme-color), 0.2);
  height: 30px;
  flex-shrink: 0;
  padding: 0 8px 2px 8px;
  font-size: 1rem;
  transition: all 0.1s ease;
  pointer-events: all;
  background: rgba(255, 255, 255, 0.2);
}

.input-data div.input {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.6rem;
  position: relative;
  height: 32px;
  pointer-events: none;
}

.input-data input {
  border: none;
  outline: none;
  border-bottom: 2px #000000;
  background: none;
  padding: 0;
  height: 100%;
  width: 100%;
  text-align: inherit;
  z-index: 10;
}

.input-data:hover {
  background: #ffffffce;
}

.underline {
  background: rgba(var(--theme-color), 1);
  margin-left: -8px;
  height: 2px;
  width: calc(100% + 16px);
  transform: scale(0, 1);
  opacity: 0;
  border-radius: var(--border-radius-medium);
  transition: transform 0.3s ease, opacity 0.3s ease;
}

input:focus~.underline {
  transform: scale(1);
  opacity: 1;
  transition: transform 0.3s ease, opacity 0.3s ease;
}

input:focus::-webkit-input-placeholder {
  color: #00000000;
}</style>