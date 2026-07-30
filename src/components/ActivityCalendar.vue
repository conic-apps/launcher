<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  data: number[];
}>();

const weeks = computed(() => {
  const result: (number | null)[][] = [];
  const totalWeeks = 52;
  const padding = new Date().getDay();

  let week: (number | null)[] = new Array(padding).fill(null);
  for (let i = 0; i < props.data.length && result.length < totalWeeks; i++) {
    week.push(props.data[i]);
    if (week.length === 7) {
      result.push(week);
      week = [];
    }
  }
  if (week.length > 0) {
    while (week.length < 7) week.push(null);
    result.push(week);
  }
  return result;
});

function getLevel(value: number | null): number {
  if (value === null || value === undefined) return 0;
  if (value === 0) return 0;
  if (value <= 2) return 1;
  if (value <= 5) return 2;
  if (value <= 10) return 3;
  return 4;
}

const monthLabels = [
  "1月",
  "2月",
  "3月",
  "4月",
  "5月",
  "6月",
  "7月",
  "8月",
  "9月",
  "10月",
  "11月",
  "12月",
];
</script>

<template>
  <div class="activity-calendar">
    <div style="overflow-x: auto; width: 100%; padding-bottom: 8px">
      <div class="months">
        <span v-for="m in monthLabels" :key="m">{{ m }}</span>
      </div>
      <div class="grid">
        <div class="week" v-for="(week, wi) in weeks" :key="wi">
          <div
            v-for="(day, di) in week"
            :key="di"
            class="cell"
            :class="`level-${getLevel(day)}`"
            :title="day !== null ? `${day} 次启动` : ''"></div>
        </div>
      </div>
    </div>
    <div class="legend">
      <span class="legend-label" style="margin-right: auto">总游戏时长：1145 小时</span>
      <span class="legend-label">少</span>
      <div class="cell level-0"></div>
      <div class="cell level-1"></div>
      <div class="cell level-2"></div>
      <div class="cell level-3"></div>
      <div class="cell level-4"></div>
      <span class="legend-label">多</span>
    </div>
  </div>
</template>

<style lang="less" scoped>
.activity-calendar {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.months {
  display: flex;
  padding-left: 28px;
  margin-bottom: 4px;

  span {
    flex: 1;
    font-size: 11px;
    color: rgba(var(--default-text-color), 0.5);
  }
}

.grid {
  display: flex;
  gap: 3px;
}

.week {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.cell {
  width: 11px;
  height: 11px;
  border-radius: 2px;
  transition: background 0.15s ease;
}

.level-0 {
  background: rgba(var(--ctp-overlay0-rgb), 0.2);
}

.level-1 {
  background: rgba(var(--ctp-green-rgb), 0.25);
}

.level-2 {
  background: rgba(var(--ctp-green-rgb), 0.45);
}

.level-3 {
  background: rgba(var(--ctp-green-rgb), 0.7);
}

.level-4 {
  background: var(--ctp-green);
}

.legend {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 3px;
  margin-top: 6px;

  .legend-label {
    font-size: 10px;
    color: rgba(var(--default-text-color), 0.45);
    margin: 0 2px;
  }

  .cell {
    width: 10px;
    height: 10px;
  }
}
</style>
