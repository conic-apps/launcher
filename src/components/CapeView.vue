<script setup lang="ts">
import { onMounted, ref, watch } from "vue";

const props = defineProps<{
  cape?: string;
}>();

const canvasRef = ref<HTMLCanvasElement>();

const BASE_CAPE_WIDTH = 64;
const BASE_SRC_X = 1;
const BASE_SRC_Y = 0;
const BASE_SRC_W = 10;
const BASE_SRC_H = 17;
const DISPLAY_SCALE = 4;
const EXPECTED_RATIO = 2;
const RATIO_TOLERANCE = 0.01;

const OUTPUT_W = BASE_SRC_W * DISPLAY_SCALE;
const OUTPUT_H = BASE_SRC_H * DISPLAY_SCALE;

function validateCapeTexture(img: HTMLImageElement): boolean {
  const ratio = img.naturalWidth / img.naturalHeight;
  if (Math.abs(ratio - EXPECTED_RATIO) > RATIO_TOLERANCE) return false;
  if (img.naturalWidth % BASE_CAPE_WIDTH !== 0) return false;
  return true;
}

async function loadAndDraw(capeUrl: string) {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const img = new Image();
  img.src = capeUrl;
  await new Promise<void>((resolve) => {
    img.onload = () => resolve();
    img.onerror = () => resolve();
  });
  if (!img.complete || img.naturalWidth === 0) return;

  if (!validateCapeTexture(img)) return;

  const scale = img.naturalWidth / BASE_CAPE_WIDTH;
  const srcX = BASE_SRC_X * scale;
  const srcY = BASE_SRC_Y * scale;
  const srcW = BASE_SRC_W * scale;
  const srcH = BASE_SRC_H * scale;

  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(img, srcX, srcY, srcW, srcH, 0, 0, canvas.width, canvas.height);
}

function startRender() {
  if (props.cape) {
    loadAndDraw(props.cape);
  } else {
    const canvas = canvasRef.value;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
  }
}

onMounted(startRender);

watch(
  () => props.cape,
  (cape) => {
    if (cape) loadAndDraw(cape);
  },
);
</script>

<template>
  <canvas ref="canvasRef" :width="OUTPUT_W" :height="OUTPUT_H" class="cape-view"></canvas>
</template>

<style scoped>
.cape-view {
  image-rendering: pixelated;
}
</style>
