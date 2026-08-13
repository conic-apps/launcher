<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="beat-map" :class="{ fill }" ref="beatMapRef">
    <canvas ref="canvasRef"></canvas>
  </div>
</template>

<script setup lang="ts">
import { getAnalyser, getAudioSampleRate, useMusicStore } from "@/store/music";
import { computed, onBeforeUnmount, onMounted, useTemplateRef } from "vue";

const BAR_COUNT = 80;
const MIN_DB = -70;
const PEAK_DECAY = 0.97;
const MIN_PEAK_LEVEL = 0.05;

const props = withDefaults(
  defineProps<{
    /** Fill the parent element instead of being a fixed-size block. */
    fill?: boolean;
    /** Skip painting an opaque background so the parent shows through. */
    transparent?: boolean;
    /** Normalize bar heights against the recent peak so quiet songs still fill the range. */
    normalize?: boolean;
    /** Smooth bars by interpolating between adjacent frequency bins. */
    interpolate?: boolean;
    /** Lower bound of the frequency range (Hz) spread across the bars. */
    minFrequency?: number;
    /** Upper bound of the frequency range (Hz) spread across the bars. */
    maxFrequency?: number;
  }>(),
  {
    fill: false,
    transparent: false,
    normalize: true,
    interpolate: true,
    minFrequency: 1800,
    maxFrequency: 3800,
  },
);

const music = useMusicStore();
const beatMapRef = useTemplateRef("beatMapRef");
const canvasRef = useTemplateRef("canvasRef");

const hasTrack = computed(() => music.currentTrack !== null);

let resizeObserver: ResizeObserver | null = null;
let animationFrameId = 0;
let frequencyData: Float32Array<ArrayBuffer> | null = null;
let peakLevel = 0;

function getCssColor(variable: string, fallback: string): string {
  const root = beatMapRef.value ?? document.documentElement;
  const style = getComputedStyle(root).getPropertyValue(variable).trim();
  return style || fallback;
}

function drawFrame() {
  const canvas = canvasRef.value;
  const container = beatMapRef.value;
  if (!canvas || !container) {
    animationFrameId = 0;
    return;
  }
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    animationFrameId = 0;
    return;
  }

  const width = canvas.width;
  const height = canvas.height;

  if (!props.transparent) {
    ctx.fillStyle = getCssColor("--ctp-mantle", "#1e1e2e");
    ctx.fillRect(0, 0, width, height);
  } else {
    ctx.clearRect(0, 0, width, height);
  }

  const barWidth = width / BAR_COUNT;
  const barGap = Math.max(1, Math.floor(barWidth * 0.25));
  const idleColor = getCssColor("--ctp-surface2", "#313244");

  const analyser = getAnalyser();
  if (analyser && hasTrack.value && music.isPlaying) {
    if (!frequencyData || frequencyData.length !== analyser.frequencyBinCount) {
      frequencyData = new Float32Array(analyser.frequencyBinCount);
    }
    analyser.getFloatFrequencyData(frequencyData);
    const nyquist = getAudioSampleRate() / 2;
    const binCount = frequencyData.length;
    const maxBin = Math.round(binCount * (props.maxFrequency / nyquist));
    const minBin = Math.min(Math.round(binCount * (props.minFrequency / nyquist)), maxBin - 1);
    const usableBins = Math.max(1, maxBin - minBin);
    const gradient = ctx.createLinearGradient(0, height, 0, 0);
    gradient.addColorStop(0, getCssColor("--ctp-lavender", "#cba6f7"));
    gradient.addColorStop(1, getCssColor("--ctp-blue", "#b4befe"));
    let frameMax = 0;
    const barValues: number[] = [];
    for (let i = 0; i < BAR_COUNT; i++) {
      let db: number;
      if (props.interpolate) {
        const binPos = minBin + (i / BAR_COUNT) * usableBins;
        const b0 = Math.min(Math.floor(binPos), usableBins - 1 + minBin);
        const b1 = Math.min(b0 + 1, usableBins - 1 + minBin);
        const frac = binPos - Math.floor(binPos);
        const db0 = frequencyData[b0];
        const db1 = frequencyData[b1];
        if (Number.isFinite(db0) && Number.isFinite(db1)) {
          db = db0 + (db1 - db0) * frac;
        } else {
          db = Number.isFinite(db1) ? db1 : db0;
        }
      } else {
        const bin = minBin + Math.min(Math.floor((i / BAR_COUNT) * usableBins), usableBins - 1);
        db = frequencyData[bin];
      }
      const value = db <= MIN_DB ? 0 : (db - MIN_DB) / -MIN_DB;
      barValues.push(value);
      if (props.normalize && value > frameMax) {
        frameMax = value;
      }
    }
    let scale = 1;
    if (props.normalize) {
      peakLevel = Math.max(frameMax, peakLevel * PEAK_DECAY, MIN_PEAK_LEVEL);
      scale = 1 / peakLevel;
    }
    for (let i = 0; i < BAR_COUNT; i++) {
      const barHeight = Math.max(Math.min(barValues[i] * scale, 1) * height, 1);
      const x = Math.floor(i * barWidth);
      ctx.fillStyle = gradient;
      ctx.fillRect(x, height - barHeight, barWidth - barGap, barHeight);
    }
  } else {
    for (let i = 0; i < BAR_COUNT; i++) {
      const x = Math.floor(i * barWidth);
      ctx.fillStyle = idleColor;
      ctx.fillRect(x, height - 2, barWidth - barGap, 2);
    }
  }

  animationFrameId = requestAnimationFrame(drawFrame);
}

function ensureAnimationLoop() {
  if (animationFrameId === 0) {
    animationFrameId = requestAnimationFrame(drawFrame);
  }
}

function resizeCanvas() {
  const container = beatMapRef.value;
  const canvas = canvasRef.value;
  if (!container || !canvas) {
    return;
  }
  const rect = container.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.max(1, Math.floor(rect.width * dpr));
  canvas.height = Math.max(1, Math.floor(rect.height * dpr));
}

onMounted(() => {
  const container = beatMapRef.value;
  const canvas = canvasRef.value;
  if (container && canvas) {
    resizeCanvas();
    resizeObserver = new ResizeObserver(() => {
      resizeCanvas();
    });
    resizeObserver.observe(container);
  }
  ensureAnimationLoop();
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  cancelAnimationFrame(animationFrameId);
});
</script>

<style lang="less" scoped>
.beat-map {
  width: 100%;
  height: 48px;
  position: relative;
  overflow: hidden;
  border-radius: 8px;
  background: var(--ctp-mantle);

  &.fill {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 0;
    background: transparent;
  }

  canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
}
</style>
