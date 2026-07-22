<script setup lang="ts">
import { computed } from "vue";

export type BarChartData = {
  label: string;
  value: number;
};

const props = defineProps<{
  data: BarChartData[];
}>();

const maxValue = computed(() => {
  return Math.max(...props.data.map((d) => d.value), 1);
});

function getBarHeight(value: number): string {
  return `${(value / maxValue.value) * 100}%`;
}
</script>

<template>
  <div class="bar-chart">
    <div class="bars">
      <div class="bar-wrapper" v-for="(item, index) in data" :key="index">
        <div class="bar-track">
          <div class="bar-value">{{ item.value }}</div>
          <div class="bar-fill" :style="{ height: getBarHeight(item.value) }"></div>
        </div>
        <div class="bar-label" :title="item.label">{{ item.label }}</div>
      </div>
    </div>
  </div>
</template>

<style lang="less" scoped>
.bar-chart {
  width: 100%;
  overflow-x: auto;
  .bars {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    height: 160px;
    padding: 0 4px;
  }

  .bar-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
    min-width: 48px;
    height: 100%;
  }

  .bar-value {
    font-size: 11px;
    color: rgba(var(--default-text-color), 0.6);
    margin-bottom: 4px;
    flex-shrink: 0;
  }

  .bar-track {
    flex: 1;
    width: 100%;
    max-width: 36px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: end;
    border-radius: 4px;
    overflow: hidden;
    background: rgba(var(--ctp-surface0-rgb), 0.3);
  }

  .bar-fill {
    width: 100%;
    background: var(--ctp-blue);
    border-radius: 4px 4px 0 0;
    transition: height 0.4s cubic-bezier(0.22, 1, 0.36, 1);
    min-height: 2px;
  }

  .bar-label {
    font-size: 11px;
    color: rgba(var(--default-text-color), 0.55);
    margin-top: 6px;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60px;
    flex-shrink: 0;
  }
}
</style>
