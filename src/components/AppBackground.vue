<script setup lang="ts">
import { useConfigStore } from "@/store/config";
import { onMounted, onBeforeUnmount, ref } from "vue";

const canvasRef = ref<HTMLCanvasElement | null>(null);

let observer: ResizeObserver | undefined;

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

  const curves = [50, 110, 180, 280];

  const range = Math.max(rect.width, rect.height) * 1.5;

  curves.forEach((a, index) => {
    const bodyElement = document.body;
    console.log(bodyElement.classList.contains("theme-Latte"));
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
});

const configStore = useConfigStore();

configStore.$subscribe(() => {
  draw();
});

onBeforeUnmount(() => {
  observer?.disconnect();
});
</script>

<template>
  <canvas ref="canvasRef" class="background" />
</template>

<style scoped>
.background {
  width: 100%;
  height: 100%;
  display: block;
  opacity: 0.4;
}
</style>
