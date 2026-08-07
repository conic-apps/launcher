<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div
    ref="containerRef"
    class="world-map"
    :class="{ 'is-dragging': dragging }"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @pointercancel="onPointerUp">
    <canvas ref="canvasRef" class="world-map-canvas"></canvas>
    <div v-if="loading" class="world-map-status">
      <ItemLoadingIcon status="in-progress"></ItemLoadingIcon>
      <span>加载地图中…</span>
    </div>
    <div v-else-if="error" class="world-map-status error">
      <ItemLoadingIcon status="error"></ItemLoadingIcon>
      <span>{{ error }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import ItemLoadingIcon from "@/components/ItemLoadingIcon.vue";
import { renderWorldMap, type WorldMapRenderResult } from "@conic/content";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    instanceId: string;
    folderName: string;
    dimension?: string;
    width?: number;
    height?: number;
    water?: boolean;
    shading?: boolean;
    altitudeShading?: boolean;
    maxScale?: number;
  }>(),
  {
    width: 512,
    height: 512,
    water: true,
    shading: true,
    altitudeShading: true,
    maxScale: 64,
  },
);

const containerRef = ref<HTMLDivElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);

const loading = ref(false);
const error = ref<string | null>(null);

// 视口尺寸（CSS 像素）
const viewportW = ref(0);
const viewportH = ref(0);

// 渲染变换：一个方块对应一个像素，scale 为每方块像素数
const scale = ref(1);
const offsetX = ref(0);
const offsetY = ref(0);

const mapSize = ref<{ width: number; height: number } | null>(null);

// 离屏画布保存原始像素，显示画布仅做变换绘制
let mapCanvas: HTMLCanvasElement | null = null;
let ctx: CanvasRenderingContext2D | null = null;
let resizeObserver: ResizeObserver | undefined;
let needsFit = false;
const dragging = ref(false);
let lastX = 0;
let lastY = 0;
let requestSeq = 0;

const fitScale = computed(() => {
  if (!mapSize.value || viewportW.value <= 0 || viewportH.value <= 0) return 1;
  return Math.min(viewportW.value / mapSize.value.width, viewportH.value / mapSize.value.height);
});

const minScale = computed(() => fitScale.value * 0.5);
const maxScale = computed(() => props.maxScale);

// 当前可见的世界区块范围（方块坐标），坐标原点为渲染请求中心
const visibleWorldRect = computed(() => {
  if (!mapSize.value) return null;
  return {
    minX: -offsetX.value / scale.value,
    minZ: -offsetY.value / scale.value,
    maxX: (viewportW.value - offsetX.value) / scale.value,
    maxZ: (viewportH.value - offsetY.value) / scale.value,
  };
});

function syncCanvasSize() {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.round(viewportW.value * dpr));
  canvas.height = Math.max(1, Math.round(viewportH.value * dpr));
}

function draw() {
  if (!ctx || !mapCanvas || !mapSize.value) return;
  const dpr = window.devicePixelRatio || 1;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, viewportW.value, viewportH.value);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(
    mapCanvas,
    offsetX.value,
    offsetY.value,
    mapSize.value.width * scale.value,
    mapSize.value.height * scale.value,
  );
}

function buildMapCanvas(result: WorldMapRenderResult) {
  const { width, height, pixels } = result;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const offscreen = canvas.getContext("2d");
  if (!offscreen) return;
  const imageData = offscreen.createImageData(width, height);
  imageData.data.set(pixels);
  offscreen.putImageData(imageData, 0, 0);
  mapCanvas = canvas;
  mapSize.value = { width, height };
}

function resetView() {
  if (!mapSize.value || viewportW.value <= 0 || viewportH.value <= 0) return;
  const s = fitScale.value;
  scale.value = s;
  offsetX.value = (viewportW.value - mapSize.value.width * s) / 2;
  offsetY.value = (viewportH.value - mapSize.value.height * s) / 2;
  needsFit = false;
  draw();
}

function zoomAt(px: number, py: number, factor: number) {
  if (!mapSize.value) return;
  const worldX = (px - offsetX.value) / scale.value;
  const worldZ = (py - offsetY.value) / scale.value;
  const next = Math.min(Math.max(scale.value * factor, minScale.value), maxScale.value);
  offsetX.value = px - worldX * next;
  offsetY.value = py - worldZ * next;
  scale.value = next;
  needsFit = false;
  draw();
}

async function loadMap() {
  const seq = ++requestSeq;
  loading.value = true;
  error.value = null;
  try {
    const result = await renderWorldMap({
      instanceId: props.instanceId,
      folderName: props.folderName,
      width: props.width,
      height: props.height,
      dimension: props.dimension,
      water: props.water,
      shading: props.shading,
      altitudeShading: props.altitudeShading,
    });
    if (seq !== requestSeq) return;
    buildMapCanvas(result);
    needsFit = true;
    resetView();
  } catch (err) {
    if (seq !== requestSeq) return;
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    if (seq === requestSeq) loading.value = false;
  }
}

function handleResize() {
  const el = containerRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  viewportW.value = rect.width;
  viewportH.value = rect.height;
  syncCanvasSize();
  if (needsFit && mapSize.value) {
    resetView();
  } else {
    draw();
  }
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  const canvas = canvasRef.value;
  if (!canvas) return;
  const rect = canvas.getBoundingClientRect();
  const factor = e.ctrlKey ? Math.exp(-e.deltaY * 0.01) : Math.exp(-e.deltaY * 0.0016);
  zoomAt(e.clientX - rect.left, e.clientY - rect.top, Math.min(Math.max(factor, 0.2), 5));
}

function onPointerDown(e: PointerEvent) {
  dragging.value = true;
  lastX = e.clientX;
  lastY = e.clientY;
  canvasRef.value?.setPointerCapture(e.pointerId);
}

function onPointerMove(e: PointerEvent) {
  if (!dragging.value) return;
  offsetX.value += e.clientX - lastX;
  offsetY.value += e.clientY - lastY;
  lastX = e.clientX;
  lastY = e.clientY;
  draw();
}

function onPointerUp() {
  dragging.value = false;
}

defineExpose({
  resetView,
  zoomAt,
  visibleWorldRect,
  scale,
  offsetX,
  offsetY,
});

onMounted(() => {
  const canvas = canvasRef.value;
  if (canvas) {
    ctx = canvas.getContext("2d");
    canvas.addEventListener("wheel", onWheel, { passive: false });
  }
  resizeObserver = new ResizeObserver(handleResize);
  if (containerRef.value) resizeObserver.observe(containerRef.value);
  handleResize();
});

onBeforeUnmount(() => {
  requestSeq++;
  resizeObserver?.disconnect();
  canvasRef.value?.removeEventListener("wheel", onWheel);
});

watch(
  () => [
    props.instanceId,
    props.folderName,
    props.dimension,
    props.width,
    props.height,
    props.water,
    props.shading,
    props.altitudeShading,
  ],
  loadMap,
  { immediate: true },
);
</script>

<style lang="less" scoped>
.world-map {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 160px;
  overflow: hidden;
  border-radius: 8px;
  background: var(--ctp-base);
  touch-action: none;
  user-select: none;

  .world-map-canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
    cursor: grab;
  }

  &.is-dragging .world-map-canvas {
    cursor: grabbing;
  }

  .world-map-status {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 12px;
    color: var(--ctp-subtext0);
    background: var(--ctp-base);

    &.error {
      color: var(--ctp-red);
    }
  }
}
</style>
