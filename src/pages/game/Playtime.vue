<template>
  <div class="playtime">
    <div class="content">
      <p class="value">2,048</p>
      <p class="title">Total Playtime</p>
      <p class="date">Aug 10, 2016 - Present</p>
    </div>
    <div class="line"></div>
    <div class="content">
      <p class="value">15h</p>
      <p class="title">Latest Playtime</p>
      <p class="date">Nov 15</p>
    </div>
    <div class="line"></div>
    <div class="content">
      <p class="value">86h</p>
      <p class="title">Longest Playtime</p>
      <p class="date">Dec 19, 2021</p>
    </div>
  </div>
</template>

<script setup lang="ts"></script>

<style lang="less" scoped>
.playtime {
  background: var(--card-background);
  display: flex;
  align-items: center;
  width: 100%;
  border: var(--card-border);
  border-radius: var(--card-border-radius);

  > div.content {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 28px 0;

    p.value {
      font-weight: bolder;
      font-size: 24px;
      margin-bottom: 16px;
    }

    p.title {
      font-size: 15px;
    }

    p.date {
      margin-top: 8px;
      opacity: 0.6;
      font-size: 14px;
    }
  }

  > div.line {
    width: 1px;
    background: rgba(255, 255, 255, 0.08);
    height: 60%;
  }
}
</style>
