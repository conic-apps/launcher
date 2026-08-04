<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  data: number[];
}>();

const COLUMN_WIDTH = 14;
const DAY_LABELS_OFFSET = 32;

const dayLabels = ["", "周一", "", "周三", "", "周五", ""];

const today = new Date();
today.setHours(0, 0, 0, 0);

const startDate = computed(() => {
  const date = new Date(today);
  date.setDate(date.getDate() - (props.data.length - 1));
  return date;
});

const leading = computed(() => {
  return startDate.value.getDay();
});

const weeks = computed(() => {
  const result: (number | null)[][] = [];
  let week: (number | null)[] = [];

  for (let i = 0; i < leading.value; i++) {
    week.push(null);
  }

  for (let i = 0; i < props.data.length; i++) {
    week.push(props.data[i]);
    if (week.length === 7) {
      result.push(week);
      week = [];
    }
  }

  if (week.length > 0) {
    result.push(week);
  }

  return result;
});

function getCellDate(flatIndex: number): Date {
  const date = new Date(today);
  date.setDate(date.getDate() - (props.data.length - 1) - leading.value + flatIndex);
  return date;
}

function formatDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

interface MonthLabel {
  text: string;
  column: number;
}

const monthLabels = computed<MonthLabel[]>(() => {
  if (props.data.length === 0) {
    return [];
  }

  const firstColumnSunday = new Date(startDate.value);
  firstColumnSunday.setDate(firstColumnSunday.getDate() - startDate.value.getDay());

  const monthOfColumn = (column: number): number => {
    const columnStart = new Date(firstColumnSunday);
    columnStart.setDate(firstColumnSunday.getDate() + column * 7);
    const firstRendered =
      columnStart.getTime() < startDate.value.getTime() ? startDate.value : columnStart;
    return firstRendered.getFullYear() * 12 + firstRendered.getMonth();
  };

  const labels: MonthLabel[] = [];
  const columnCount = weeks.value.length;
  let runStart = 0;
  let runMonth = monthOfColumn(0);

  for (let column = 1; column <= columnCount; column++) {
    const nextMonth = column < columnCount ? monthOfColumn(column) : -1;
    if (nextMonth !== runMonth) {
      if (column - runStart >= 2) {
        labels.push({
          text: `${(runMonth % 12) + 1}月`,
          column: runStart,
        });
      }
      runStart = column;
      runMonth = nextMonth;
    }
  }

  return labels;
});

const totalLaunches = computed(() => {
  return props.data.reduce((sum, count) => sum + count, 0);
});

function getLevel(value: number | null): number {
  if (value === null || value === undefined) return 0;
  if (value === 0) return 0;
  if (value <= 2) return 1;
  if (value <= 5) return 2;
  if (value <= 10) return 3;
  return 4;
}
</script>

<template>
  <div class="activity-calendar">
    <div style="overflow-x: auto; overflow-y: hidden; width: 100%; padding-bottom: 8px">
      <div class="months">
        <span
          v-for="label in monthLabels"
          :key="label.column"
          :style="{ left: `${DAY_LABELS_OFFSET + label.column * COLUMN_WIDTH}px` }"
          >{{ label.text }}</span
        >
      </div>
      <div class="calendar-row">
        <div class="day-labels">
          <span v-for="(label, i) in dayLabels" :key="i">{{ label }}</span>
        </div>
        <div class="grid">
          <div class="week" v-for="(week, wi) in weeks" :key="wi">
            <div
              v-for="(day, di) in week"
              :key="di"
              class="cell"
              :class="`level-${getLevel(day)}`"
              :title="
                day !== null ? `${formatDate(getCellDate(wi * 7 + di))} · ${day} 次启动` : ''
              "></div>
          </div>
        </div>
      </div>
    </div>
    <div class="legend">
      <span class="legend-label" style="margin-right: auto"
        >最近一年内使用此账户启动 {{ totalLaunches }} 次游戏</span
      >
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
  position: relative;
  height: 14px;
  margin-bottom: 4px;
  white-space: nowrap;

  span {
    position: absolute;
    top: 0;
    font-size: 11px;
    color: rgba(var(--default-text-color), 0.5);
  }
}

.grid {
  display: flex;
  gap: 3px;
}

.calendar-row {
  display: flex;
}

.day-labels {
  display: flex;
  flex-direction: column;
  gap: 3px;
  width: 28px;
  margin-right: 4px;
  flex-shrink: 0;

  span {
    height: 11px;
    line-height: 11px;
    font-size: 11px;
    text-align: right;
    color: rgba(var(--default-text-color), 0.5);
  }
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
