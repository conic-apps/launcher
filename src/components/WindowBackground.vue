<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<script setup lang="ts">
import { useConfigStore } from "@/store/config";
import { onMounted, onBeforeUnmount, ref } from "vue";
import gsap from "gsap";

const canvasRef = ref<HTMLCanvasElement | null>(null);
const wrapperRef = ref<HTMLDivElement | null>(null);

let observer: ResizeObserver | undefined;
let moveX: ((value: number) => void) | undefined;
let moveY: ((value: number) => void) | undefined;

const MAX_OFFSET = 16;
const SCALE = 1.08;

function onMouseMove(event: MouseEvent) {
  const nx = (event.clientX - window.innerWidth / 2) / (window.innerWidth / 2);
  const ny = (event.clientY - window.innerHeight / 2) / (window.innerHeight / 2);

  moveX?.(nx * MAX_OFFSET);
  moveY?.(ny * MAX_OFFSET);
}

function onMouseLeave() {
  moveX?.(0);
  moveY?.(0);
}

function drawHyperbola(ctx: CanvasRenderingContext2D, a: number, b: number, range: number) {
  const step = 1;

  function drawBranch(signX: number, signY: number) {
    ctx.beginPath();

    let first = true;

    for (let x = a; x <= range; x += step) {
      const realX = signX * x;

      const y = signY * b * Math.sqrt((x * x) / (a * a) - 1);

      if (!Number.isFinite(y)) {
        continue;
      }

      if (first) {
        ctx.moveTo(realX, y);
        first = false;
      } else {
        ctx.lineTo(realX, y);
      }
    }

    ctx.stroke();
  }

  // 四个分支
  drawBranch(1, 1);
  drawBranch(1, -1);
  drawBranch(-1, 1);
  drawBranch(-1, -1);
}

function draw() {
  const canvas = canvasRef.value;

  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();

  const dpr = window.devicePixelRatio || 1;

  canvas.width = rect.width * dpr;

  canvas.height = rect.height * dpr;

  const ctx = canvas.getContext("2d");

  if (!ctx) return;

  ctx.scale(dpr, dpr);

  ctx.clearRect(0, 0, rect.width, rect.height);

  ctx.save();

  // 坐标中心
  ctx.translate(rect.width / 2, rect.height / 2);

  // 主轴方向
  ctx.rotate((-40 * Math.PI) / 180);

  ctx.lineWidth = 0.6;

  const b = 200;

  const curves = [40, 110, 180, 280, 420, 640];

  const range = Math.max(rect.width, rect.height) * 1.5;

  curves.forEach((a, index) => {
    const bodyElement = document.body;
    if (bodyElement.classList.contains("theme-Latte")) {
      ctx.strokeStyle = `rgba(0,0,0,${0.45 - index * 0.055})`;
    } else {
      ctx.strokeStyle = `rgba(255,255,255,${0.45 - index * 0.055})`;
    }

    drawHyperbola(ctx, a, b, range);
  });

  // 中心点
  // ctx.fillStyle = "rgba(255,255,255,.8)";
  //
  // ctx.beginPath();
  //
  // ctx.arc(0, 0, 3, 0, Math.PI * 2);
  //
  // ctx.fill();

  ctx.restore();
}

onMounted(() => {
  observer = new ResizeObserver(draw);

  if (canvasRef.value) {
    observer.observe(canvasRef.value);
  }

  draw();

  if (wrapperRef.value) {
    gsap.set(wrapperRef.value, {
      transformOrigin: "center",
      scale: SCALE,
      x: 0,
      y: 0,
    });

    moveX = gsap.quickTo(wrapperRef.value, "x", { duration: 0.05, ease: "power3.out" });
    moveY = gsap.quickTo(wrapperRef.value, "y", { duration: 0.05, ease: "power3.out" });
  }

  window.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseleave", onMouseLeave);
});

const configStore = useConfigStore();

configStore.$subscribe(() => {
  draw();
});

onBeforeUnmount(() => {
  observer?.disconnect();
  window.removeEventListener("mousemove", onMouseMove);
  document.removeEventListener("mouseleave", onMouseLeave);

  if (wrapperRef.value) {
    gsap.killTweensOf(wrapperRef.value);
  }
});
</script>

<template>
  <div ref="wrapperRef" class="background-wrapper">
    <canvas ref="canvasRef" class="background" />
  </div>
</template>

<style scoped>
.background-wrapper {
  width: 100%;
  height: 100%;
  .background {
    width: 100%;
    height: 100%;
    display: block;
    opacity: 0.3;
  }
}
</style>
