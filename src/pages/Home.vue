<template>
  <div class="home-page-main">home</div>
</template>

<script setup lang="ts"></script>

<style lang="less" scoped>
.game-page-main {
  width: 100%;
  height: 100%;
  display: flex;
}

.row-1,
.row-2 {
  height: 100%;
  display: flex;
  flex-direction: column;
  // border: 1px solid #fff;
}

.row-1 {
  padding: 24px 24px;
  width: 100%;
}

.row-2 {
  width: 328px;
  padding: 24px 24px 24px 0;
  flex-shrink: 0;
}

// todo: move to main.css
.group-name {
  width: 100%;
  height: 32px;
  font-size: 14.5px;
  margin-bottom: 8px;
}

.group-button {
  background: rgba(255, 255, 255, 0.08);
  border: none;
  border-radius: 4px;
  height: 20px;
  width: 20px;
  font-size: 10px;
}

.group-button:hover {
  background: rgba(255, 255, 255, 0.12);
}

.group-button:active {
  background: rgba(255, 255, 255, 0.08);
}

.group-button i::before {
  transform: scale(0.7);
}
</style>
