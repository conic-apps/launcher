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
    @pointercancel="onPointerUp"
    @pointerleave="onPointerLeave">
    <canvas ref="canvasRef" class="world-map-canvas"></canvas>
    <div v-if="loading" class="world-map-status">
      <ItemLoadingIcon status="in-progress"></ItemLoadingIcon>
      <span>加载地图中…</span>
    </div>
    <div v-else-if="error" class="world-map-status error">
      <ItemLoadingIcon status="error"></ItemLoadingIcon>
      <span>{{ error }}</span>
    </div>
    <div v-if="DEBUG_SHOW_TILE_CACHE_COUNT || DEBUG_SHOW_TILE_CACHE_STATS" class="world-map-debug">
      <div v-if="DEBUG_SHOW_TILE_CACHE_COUNT">Render: {{ renderCacheCount }}</div>
      <div v-if="DEBUG_SHOW_TILE_CACHE_STATS">PNG: {{ pngCacheCount }}</div>
    </div>
    <div
      v-if="showCursorCoords && cursorX !== null && cursorZ !== null"
      class="world-map-coords"
      :class="cursorCoordsPositionClass">
      x: {{ cursorX }}, z: {{ cursorZ }}
    </div>
  </div>
</template>

<script setup lang="ts">
import ItemLoadingIcon from "@/components/ItemLoadingIcon.vue";
import { renderWorldMap } from "@conic/content";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

const MAX_CONCURRENT = 24;
const TILE_LOAD_DEBOUNCE = 50;
const INITIAL_VIEW_BLOCKS = 512;
const FADE_MS = 100;
// 渲染缓存保留范围：可视区域向外扩展的圈数
const TILE_RENDER_MARGIN = 6;
// 调试开关：右上角显示渲染缓存中的 ImageBitmap 数量
const DEBUG_SHOW_TILE_CACHE_COUNT = true;
// 调试开关：右上角显示 PNG 数据缓存中的 tile 数量
const DEBUG_SHOW_TILE_CACHE_STATS = true;

// 光标坐标显示位置
type CursorCoordsPosition =
  | "bottom-center"
  | "top-center"
  | "top-left"
  | "top-right"
  | "bottom-left"
  | "bottom-right";

const props = withDefaults(
  defineProps<{
    instanceId: string;
    folderName: string;
    dimension?: string;
    tileSize?: number;
    centerX?: number;
    centerZ?: number;
    water?: boolean;
    shading?: boolean;
    altitudeShading?: boolean;
    minScale?: number;
    maxScale?: number;
    showCursorCoords?: boolean;
    cursorCoordinatesPosition?: CursorCoordsPosition;
  }>(),
  {
    tileSize: 64,
    water: true,
    shading: true,
    altitudeShading: true,
    minScale: 0.25,
    maxScale: 64,
    showCursorCoords: false,
    cursorCoordinatesPosition: "bottom-center",
  },
);

const containerRef = ref<HTMLDivElement | null>(null);
const canvasRef = ref<HTMLCanvasElement | null>(null);

const error = ref<string | null>(null);
const tilesLoaded = ref(0);

// 视口尺寸（CSS 像素）
const viewportW = ref(0);
const viewportH = ref(0);

// 渲染变换：一个方块对应一个像素，scale 为每方块像素数
const scale = ref(1);
const offsetX = ref(0);
const offsetY = ref(0);

// PNG 数据缓存（长期）：key 为 "tx,tz"，value 为后端返回的 PNG 二进制
const pngCache = new Map<string, Uint8Array<ArrayBuffer>>();
// ImageBitmap 渲染缓存（短期）：仅保留可视区域外扩 TILE_RENDER_MARGIN 圈
const renderCache = new Map<string, ImageBitmap>();
const tileAppear = new Map<string, number>();
const inFlight = new Set<string>();
const pending = new Set<string>();
const decoding = new Set<string>();
const decodeQueue: string[] = [];
const tilesFailed = new Set<string>();

// 缓存为非响应式 Map，通过 tick 触发下方计数重新计算
const cacheStatsTick = ref(0);

const renderCacheCount = computed(() => {
  void cacheStatsTick.value;
  return renderCache.size;
});

const pngCacheCount = computed(() => {
  void cacheStatsTick.value;
  return pngCache.size;
});

function bumpCacheStats() {
  cacheStatsTick.value++;
}

let ctx: CanvasRenderingContext2D | null = null;
let resizeObserver: ResizeObserver | undefined;
let tileLoadTimer: number | undefined;
let fadeRAF: number | undefined;
const dragging = ref(false);
let lastX = 0;
let lastY = 0;
let requestSeq = 0;
let pendingInit = true;

// 光标所在方块坐标（null 表示鼠标不在地图上）
const cursorX = ref<number | null>(null);
const cursorZ = ref<number | null>(null);

const cursorCoordsPositionClass = computed(() => `is-${props.cursorCoordinatesPosition}`);

// 复用现有渲染变换（与 zoomAt / visibleWorldRect 同一套换算），
// 屏幕坐标 → 世界坐标 → 方块坐标
function screenToBlock(px: number, py: number): { x: number; z: number } | null {
  if (viewportW.value <= 0 || viewportH.value <= 0) return null;
  return {
    x: Math.floor((px - offsetX.value) / scale.value),
    z: Math.floor((py - offsetY.value) / scale.value),
  };
}

function updateCursorCoords(e: PointerEvent) {
  if (!props.showCursorCoords) return;
  const el = containerRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  const block = screenToBlock(e.clientX - rect.left, e.clientY - rect.top);
  if (block === null) {
    cursorX.value = null;
    cursorZ.value = null;
    return;
  }
  cursorX.value = block.x;
  cursorZ.value = block.z;
}

function onPointerLeave() {
  cursorX.value = null;
  cursorZ.value = null;
}

const hasTiles = computed(() => tilesLoaded.value > 0);
const loading = computed(() => !hasTiles.value && !error.value);

// 当前可见的世界区块范围（方块坐标）
const visibleWorldRect = computed(() => {
  if (viewportW.value <= 0 || viewportH.value <= 0) return null;
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
  if (!ctx || viewportW.value <= 0 || viewportH.value <= 0) return;
  const range = visibleTileRange();
  if (!range) return;
  const dpr = window.devicePixelRatio || 1;
  const s = scale.value;
  const now = performance.now();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, viewportW.value, viewportH.value);
  ctx.imageSmoothingEnabled = false;
  const T = props.tileSize;
  let fading = false;
  for (let tx = range.tx0; tx <= range.tx1; tx++) {
    for (let tz = range.tz0; tz <= range.tz1; tz++) {
      const key = `${tx},${tz}`;
      const bitmap = renderCache.get(key);
      if (!bitmap) continue;
      const appear = tileAppear.get(key);
      let alpha = 1;
      if (appear !== undefined) {
        const elapsed = now - appear;
        if (elapsed >= FADE_MS) {
          tileAppear.delete(key);
        } else {
          alpha = elapsed / FADE_MS;
          fading = true;
        }
      }
      if (alpha <= 0) continue;
      if (alpha < 1) ctx.globalAlpha = alpha;
      ctx.drawImage(bitmap, tx * T * s + offsetX.value, tz * T * s + offsetY.value, T * s, T * s);
      ctx.globalAlpha = 1;
    }
  }
  if (fading && fadeRAF === undefined) {
    fadeRAF = requestAnimationFrame(() => {
      fadeRAF = undefined;
      draw();
    });
  }
}

function base64ToBytes(b64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(b64);
  const buffer = new ArrayBuffer(binary.length);
  const bytes = new Uint8Array(buffer);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function resetView() {
  if (viewportW.value <= 0 || viewportH.value <= 0) {
    pendingInit = true;
    return;
  }
  const s = Math.min(
    Math.max(Math.min(viewportW.value, viewportH.value) / INITIAL_VIEW_BLOCKS, props.minScale),
    props.maxScale,
  );
  const cx = props.centerX ?? 0;
  const cz = props.centerZ ?? 0;
  scale.value = s;
  offsetX.value = viewportW.value / 2 - cx * s;
  offsetY.value = viewportH.value / 2 - cz * s;
  pendingInit = false;
  draw();
  dispatchTileLoads();
}

function zoomAt(px: number, py: number, factor: number) {
  if (viewportW.value <= 0 || viewportH.value <= 0) return;
  const worldX = (px - offsetX.value) / scale.value;
  const worldZ = (py - offsetY.value) / scale.value;
  const next = Math.min(Math.max(scale.value * factor, props.minScale), props.maxScale);
  offsetX.value = px - worldX * next;
  offsetY.value = py - worldZ * next;
  scale.value = next;
  draw();
  scheduleTileLoad();
}

function scheduleTileLoad() {
  // pngCache 命中立即恢复，不等待防抖
  dispatchCacheHits();
  if (tileLoadTimer !== undefined) window.clearTimeout(tileLoadTimer);
  tileLoadTimer = window.setTimeout(() => {
    tileLoadTimer = undefined;
    dispatchTileLoads();
  }, TILE_LOAD_DEBOUNCE);
}

function visibleTileRange() {
  const rect = visibleWorldRect.value;
  if (!rect) return null;
  const T = props.tileSize;
  return {
    tx0: Math.floor(rect.minX / T),
    tx1: Math.floor(rect.maxX / T),
    tz0: Math.floor(rect.minZ / T),
    tz1: Math.floor(rect.maxZ / T),
  };
}

function dispatchCacheHits() {
  const range = visibleTileRange();
  if (!range) return;
  for (let tx = range.tx0; tx <= range.tx1; tx++) {
    for (let tz = range.tz0; tz <= range.tz1; tz++) {
      const key = `${tx},${tz}`;
      if (
        renderCache.has(key) ||
        decoding.has(key) ||
        decodeQueue.includes(key) ||
        tilesFailed.has(key)
      )
        continue;
      if (pngCache.has(key)) {
        decodeQueue.push(key);
      }
    }
  }
  pumpDecodes();
}

function dispatchTileLoads() {
  const range = visibleTileRange();
  if (!range) return;
  const seq = requestSeq;
  dispatchCacheHits();
  pruneRenderCache();
  for (let tx = range.tx0; tx <= range.tx1; tx++) {
    for (let tz = range.tz0; tz <= range.tz1; tz++) {
      const key = `${tx},${tz}`;
      if (
        renderCache.has(key) ||
        inFlight.has(key) ||
        pending.has(key) ||
        decoding.has(key) ||
        decodeQueue.includes(key) ||
        tilesFailed.has(key)
      )
        continue;
      pending.add(key);
    }
  }
  pumpRequests(seq);
}

function pumpRequests(seq: number) {
  if (seq !== requestSeq) return;
  const rect = visibleWorldRect.value;
  const cx = rect ? (rect.minX + rect.maxX) / 2 : 0;
  const cz = rect ? (rect.minZ + rect.maxZ) / 2 : 0;
  const T = props.tileSize;
  while (inFlight.size < MAX_CONCURRENT && pending.size > 0) {
    let best: string | null = null;
    let bestDist = Infinity;
    for (const key of pending) {
      const [tx, tz] = key.split(",").map(Number);
      const dx = (tx + 0.5) * T - cx;
      const dz = (tz + 0.5) * T - cz;
      const d = dx * dx + dz * dz;
      if (d < bestDist) {
        bestDist = d;
        best = key;
      }
    }
    if (best === null) break;
    pending.delete(best);
    inFlight.add(best);
    const [tx, tz] = best.split(",").map(Number);
    void requestTile(best, tx, tz, seq);
  }
}

function pruneRenderCache() {
  const range = visibleTileRange();
  if (!range) return;
  for (const [key, bitmap] of renderCache) {
    const [tx, tz] = key.split(",").map(Number);
    if (
      tx < range.tx0 - TILE_RENDER_MARGIN ||
      tx > range.tx1 + TILE_RENDER_MARGIN ||
      tz < range.tz0 - TILE_RENDER_MARGIN ||
      tz > range.tz1 + TILE_RENDER_MARGIN
    ) {
      bitmap.close();
      renderCache.delete(key);
      tileAppear.delete(key);
    }
  }
  bumpCacheStats();
}

function queueDecode(key: string) {
  if (
    renderCache.has(key) ||
    decoding.has(key) ||
    decodeQueue.includes(key) ||
    tilesFailed.has(key)
  )
    return;
  decodeQueue.push(key);
  pumpDecodes();
}

function pumpDecodes() {
  while (decoding.size < MAX_CONCURRENT && decodeQueue.length > 0) {
    const key = decodeQueue.shift();
    if (key === undefined) break;
    decoding.add(key);
    void decodeTile(key);
  }
}

async function decodeTile(key: string) {
  const seq = requestSeq;
  try {
    const bytes = pngCache.get(key);
    if (!bytes) return;
    const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
    if (seq !== requestSeq) {
      bitmap.close();
      return;
    }
    renderCache.set(key, bitmap);
    tilesLoaded.value++;
    error.value = null;
    bumpCacheStats();
    pruneRenderCache();
    draw();
  } catch (err) {
    if (seq !== requestSeq) return;
    tilesFailed.add(key);
    if (tilesLoaded.value === 0) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  } finally {
    decoding.delete(key);
    if (seq === requestSeq) pumpDecodes();
  }
}

function closeRenderCache() {
  for (const bitmap of renderCache.values()) {
    bitmap.close();
  }
  renderCache.clear();
  bumpCacheStats();
}

async function requestTile(key: string, tx: number, tz: number, seq: number) {
  try {
    const result = await renderWorldMap({
      instanceId: props.instanceId,
      folderName: props.folderName,
      width: props.tileSize,
      height: props.tileSize,
      centerX: (tx + 0.5) * props.tileSize,
      centerZ: (tz + 0.5) * props.tileSize,
      dimension: props.dimension,
      water: props.water,
      shading: props.shading,
      altitudeShading: props.altitudeShading,
    });
    if (seq !== requestSeq) return;
    pngCache.set(key, base64ToBytes(result.png));
    bumpCacheStats();
    error.value = null;
    // 新 tile：首次显示走 fade 淡入
    tileAppear.set(key, performance.now());
    queueDecode(key);
  } catch (err) {
    if (seq !== requestSeq) return;
    tilesFailed.add(key);
    if (tilesLoaded.value === 0) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  } finally {
    if (seq !== requestSeq) return;
    inFlight.delete(key);
    pumpRequests(requestSeq);
  }
}

function handleResize() {
  const el = containerRef.value;
  if (!el) return;
  const rect = el.getBoundingClientRect();
  viewportW.value = rect.width;
  viewportH.value = rect.height;
  syncCanvasSize();
  if (pendingInit) {
    resetView();
  } else {
    draw();
    scheduleTileLoad();
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
  updateCursorCoords(e);
  if (!dragging.value) return;
  offsetX.value += e.clientX - lastX;
  offsetY.value += e.clientY - lastY;
  lastX = e.clientX;
  lastY = e.clientY;
  draw();
  scheduleTileLoad();
}

function onPointerUp(e: PointerEvent) {
  dragging.value = false;
  if (e.pointerType === "touch") {
    cursorX.value = null;
    cursorZ.value = null;
  }
}

defineExpose({
  resetView,
  zoomAt,
  visibleWorldRect,
  scale,
  offsetX,
  offsetY,
  loadedTileCount: tilesLoaded,
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
  if (tileLoadTimer !== undefined) window.clearTimeout(tileLoadTimer);
  if (fadeRAF !== undefined) cancelAnimationFrame(fadeRAF);
  closeRenderCache();
  pngCache.clear();
  decodeQueue.length = 0;
  decoding.clear();
  resizeObserver?.disconnect();
  canvasRef.value?.removeEventListener("wheel", onWheel);
});

function resetWorld() {
  requestSeq++;
  if (fadeRAF !== undefined) {
    cancelAnimationFrame(fadeRAF);
    fadeRAF = undefined;
  }
  closeRenderCache();
  pngCache.clear();
  bumpCacheStats();
  tileAppear.clear();
  decodeQueue.length = 0;
  decoding.clear();
  inFlight.clear();
  pending.clear();
  tilesFailed.clear();
  tilesLoaded.value = 0;
  error.value = null;
  resetView();
}

watch(
  () => [
    props.instanceId,
    props.folderName,
    props.dimension,
    props.tileSize,
    props.centerX,
    props.centerZ,
    props.water,
    props.shading,
    props.altitudeShading,
  ],
  resetWorld,
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

  .world-map-debug {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
    padding: 6px 10px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--ctp-text);
    background: var(--ctp-mantle);
    border: 1px solid var(--ctp-surface0);
    border-radius: 6px;
    pointer-events: none;
    user-select: none;
  }

  .world-map-coords {
    position: absolute;
    z-index: 2;
    padding: 4px 10px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--ctp-text);
    background: rgba(var(--ctp-mantle-rgb), 0.7);
    border: 1px solid var(--ctp-surface0);
    border-radius: 6px;
    pointer-events: none;
    user-select: none;

    &.is-bottom-center {
      left: 50%;
      bottom: 8px;
      transform: translateX(-50%);
    }

    &.is-top-center {
      left: 50%;
      top: 8px;
      transform: translateX(-50%);
    }

    &.is-top-left {
      top: 8px;
      left: 8px;
    }

    &.is-top-right {
      top: 8px;
      right: 8px;
    }

    &.is-bottom-left {
      bottom: 8px;
      left: 8px;
    }

    &.is-bottom-right {
      bottom: 8px;
      right: 8px;
    }
  }
}
</style>
