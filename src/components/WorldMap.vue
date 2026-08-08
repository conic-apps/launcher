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

const MAX_CONCURRENT = 24;
const TILE_LOAD_DEBOUNCE = 150;
const INITIAL_VIEW_BLOCKS = 512;
const FADE_MS = 200;

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
  }>(),
  {
    tileSize: 64,
    water: true,
    shading: true,
    altitudeShading: true,
    minScale: 0.25,
    maxScale: 64,
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

// 本地瓦片缓存：key 为 "tx,tz"，value 为离屏 canvas
const tileCache = new Map<string, HTMLCanvasElement>();
const tileAppear = new Map<string, number>();
const inFlight = new Set<string>();
const pending = new Set<string>();
const tilesFailed = new Set<string>();

let ctx: CanvasRenderingContext2D | null = null;
let resizeObserver: ResizeObserver | undefined;
let tileLoadTimer: number | undefined;
let fadeRAF: number | undefined;
const dragging = ref(false);
let lastX = 0;
let lastY = 0;
let requestSeq = 0;
let pendingInit = true;

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
  const dpr = window.devicePixelRatio || 1;
  const s = scale.value;
  const now = performance.now();
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, viewportW.value, viewportH.value);
  ctx.imageSmoothingEnabled = false;
  let fading = false;
  for (const [key, canvas] of tileCache) {
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
    const [tx, tz] = key.split(",").map(Number);
    if (alpha < 1) ctx.globalAlpha = alpha;
    ctx.drawImage(
      canvas,
      tx * props.tileSize * s + offsetX.value,
      tz * props.tileSize * s + offsetY.value,
      props.tileSize * s,
      props.tileSize * s,
    );
    ctx.globalAlpha = 1;
  }
  if (fading && fadeRAF === undefined) {
    fadeRAF = requestAnimationFrame(() => {
      fadeRAF = undefined;
      draw();
    });
  }
}

function base64ToBytes(b64: string): Uint8Array {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function buildTileCanvas(result: WorldMapRenderResult): HTMLCanvasElement {
  const { width, height, pixels } = result;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const offscreen = canvas.getContext("2d");
  if (offscreen) {
    const imageData = offscreen.createImageData(width, height);
    imageData.data.set(base64ToBytes(pixels));
    offscreen.putImageData(imageData, 0, 0);
  }
  return canvas;
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
  if (tileLoadTimer !== undefined) window.clearTimeout(tileLoadTimer);
  tileLoadTimer = window.setTimeout(() => {
    tileLoadTimer = undefined;
    dispatchTileLoads();
  }, TILE_LOAD_DEBOUNCE);
}

function dispatchTileLoads() {
  const rect = visibleWorldRect.value;
  if (!rect) return;
  const seq = requestSeq;
  const T = props.tileSize;
  const tx0 = Math.floor(rect.minX / T);
  const tx1 = Math.floor(rect.maxX / T);
  const tz0 = Math.floor(rect.minZ / T);
  const tz1 = Math.floor(rect.maxZ / T);

  for (let tx = tx0; tx <= tx1; tx++) {
    for (let tz = tz0; tz <= tz1; tz++) {
      const key = `${tx},${tz}`;
      if (tileCache.has(key) || inFlight.has(key) || pending.has(key) || tilesFailed.has(key))
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
    const canvas = buildTileCanvas(result);
    tileCache.set(key, canvas);
    tileAppear.set(key, performance.now());
    tilesLoaded.value++;
    error.value = null;
  } catch (err) {
    if (seq !== requestSeq) return;
    tilesFailed.add(key);
    if (tilesLoaded.value === 0) {
      error.value = err instanceof Error ? err.message : String(err);
    }
  } finally {
    if (seq !== requestSeq) return;
    inFlight.delete(key);
    draw();
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
  if (!dragging.value) return;
  offsetX.value += e.clientX - lastX;
  offsetY.value += e.clientY - lastY;
  lastX = e.clientX;
  lastY = e.clientY;
  draw();
  scheduleTileLoad();
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
  resizeObserver?.disconnect();
  canvasRef.value?.removeEventListener("wheel", onWheel);
});

function resetWorld() {
  requestSeq++;
  if (fadeRAF !== undefined) {
    cancelAnimationFrame(fadeRAF);
    fadeRAF = undefined;
  }
  tileCache.clear();
  tileAppear.clear();
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
}
</style>
