<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import gsap from "gsap";

// ============================================================
// 可调参数
// ============================================================

// 摄像机前进速度（方块 / 秒）。越大移动越快。
const CAMERA_SPEED = 6;

// 世界渲染视距（方块数）。越远性能开销越大。
const VIEW_DISTANCE = 120;

// 水平视野（度）
const FOV_DEGREES = 60;

// 双曲线背景对称中心的纵向位置（窗口高度百分比，0 = 顶部，1 = 底部）。
// 摄像机水平直视时地平线在窗口中央（50%），天空占上半屏，其中心恰好在 25% 处。
const HYPERBOLA_CENTER_Y_PERCENT = 0.25;

// 树木生成密度：单个方块列出现树木的概率（0~1，越大越密）。
const TREE_DENSITY = 0.0001;

// 树木形状：每个元素为 [dx, dy, dz]，表示相对树干底部方块
// （地表方块上方一格）的偏移。改这里即可调整树木形状与大小。
const TREE_SHAPE: ReadonlyArray<readonly [number, number, number]> = [
  // 树干（高 4 格）
  [0, 0, 0],
  [0, 1, 0],
  [0, 2, 0],
  [0, 3, 0],
  // 树叶下层（5x5 去掉四角）
  [-2, 4, -1],
  [-1, 4, -1],
  [0, 4, -1],
  [1, 4, -1],
  [2, 4, -1],
  [-2, 4, 0],
  [-1, 4, 0],
  [0, 4, 0],
  [1, 4, 0],
  [2, 4, 0],
  [-2, 4, 1],
  [-1, 4, 1],
  [0, 4, 1],
  [1, 4, 1],
  [2, 4, 1],
  [-1, 4, -2],
  [0, 4, -2],
  [1, 4, -2],
  [-1, 4, 2],
  [0, 4, 2],
  [1, 4, 2],
  // 树叶上层（3x3）
  [-1, 5, -1],
  [0, 5, -1],
  [1, 5, -1],
  [-1, 5, 0],
  [0, 5, 0],
  [1, 5, 0],
  [-1, 5, 1],
  [0, 5, 1],
  [1, 5, 1],
];

// 世界方块描边透明度（0~1，越大越明显）
const WORLD_STROKE_ALPHA = 0.55;

// 世界方块描边线宽（CSS 像素）
const WORLD_LINE_WIDTH = 0.6;

// 投影到屏幕上高度小于该值（像素）的方块面将被整体跳过，用于远处降噪
const MIN_FACE_PX = 1.2;

// 地形基准高度（地表方块所在 y）
const BASE_HEIGHT = 4;

// 丘陵起伏幅度（方块）
const HILL_AMPLITUDE = 2.2;

// 摄像机高度 = 基准地表高度 + 该值
const EYE_HEIGHT = 5.6;

// 鼠标视差（沿用原实现）
const MAX_OFFSET = 16;
const SCALE = 1.08;

// 八个角点相对方块坐标的偏移，下标即角点编号
const CORNER_OFFSETS: ReadonlyArray<readonly [number, number, number]> = [
  [0, 0, 0],
  [1, 0, 0],
  [0, 1, 0],
  [1, 1, 0],
  [0, 0, 1],
  [1, 0, 1],
  [0, 1, 1],
  [1, 1, 1],
];

// 六个面各自的角点顺序（均为合法四边形）
const FACE_NZ = [0, 1, 3, 2]; // z 面（z=0 平面，面向摄像机）
const FACE_NX = [0, 2, 6, 4]; // -x 面
const FACE_PX = [1, 5, 7, 3]; // +x 面
const FACE_PY = [2, 3, 7, 6]; // 顶面
const FACE_NY = [0, 1, 5, 4]; // 底面

const MAX_HALF_X = Math.ceil(VIEW_DISTANCE * Math.tan((FOV_DEGREES * Math.PI) / 360)) + 4;

// 摄像机（x = -0.5，使摄像机位于方块中心而非方块边上）
const CAM_X = -0.5;
const camY = BASE_HEIGHT + EYE_HEIGHT;
let camZ = 0;

// 画布 / 几何状态
const canvasRef = ref<HTMLCanvasElement | null>(null);
const wrapperRef = ref<HTMLDivElement | null>(null);

let moveX: ((value: number) => void) | undefined;
let moveY: ((value: number) => void) | undefined;

let viewW = 0;
let viewH = 0;
let halfW = 0;
let halfH = 0;
let focal = 0;
let gctx: CanvasRenderingContext2D | null = null;

let fillColor = "rgb(30,30,46)";
let latte = false;

// 每帧的噪声高度缓存
const heightCache = new Map<number, number>();

// 当前可见区域内树木占用的方块集合（key 为 "x,y,z"）
const treeBlocks = new Set<string>();

// 双曲线天空离屏缓存
let skyCanvas: HTMLCanvasElement | null = null;
let skyThemeKey = "";

// 列绘制顺序：|x - camX| 从大到小（远处先画），保证画家算法正确遮挡
const xOrder: number[] = [];
{
  const xMin0 = Math.floor(CAM_X - MAX_HALF_X);
  const xMax0 = Math.ceil(CAM_X + MAX_HALF_X);
  for (let x = xMin0; x <= xMax0; x++) {
    xOrder.push(x);
  }
  xOrder.sort((a, b) => Math.abs(b - CAM_X) - Math.abs(a - CAM_X));
}

let raf = 0;
let lastTime = 0;

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

// ============================================================
// 确定性地形生成（同一 (x, z) 永远得到相同结果，便于摄像机持续前进）
// ============================================================

function hash2i(ix: number, iz: number, seed: number): number {
  let h = Math.imul(ix | 0, 374761393) ^ Math.imul(iz | 0, 668265263);
  h = (h + seed) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

function valueNoise(x: number, z: number, seed: number): number {
  const ix = Math.floor(x);
  const iz = Math.floor(z);
  const fx = x - ix;
  const fz = z - iz;
  const ux = fx * fx * (3 - 2 * fx);
  const uz = fz * fz * (3 - 2 * fz);
  const a = hash2i(ix, iz, seed);
  const b = hash2i(ix + 1, iz, seed);
  const c = hash2i(ix, iz + 1, seed);
  const d = hash2i(ix + 1, iz + 1, seed);
  return a + (b - a) * ux + (c - a) * uz + (a - b - c + d) * ux * uz;
}

function terrainHeight(x: number, z: number): number {
  const n1 = (valueNoise(x / 14, z / 14, 0x71) - 0.5) * 2;
  const n2 = (valueNoise(x / 4.5, z / 4.5, 0x3a) - 0.5) * 1.2;
  const h = Math.round(BASE_HEIGHT + n1 * HILL_AMPLITUDE + n2);
  return Math.max(2, Math.min(7, h));
}

function heightAt(x: number, z: number): number {
  const key = x + z * 4096;
  let v = heightCache.get(key);
  if (v === undefined) {
    v = terrainHeight(x, z);
    heightCache.set(key, v);
  }
  return v;
}

function treeKey(x: number, y: number, z: number): string {
  return x + "," + y + "," + z;
}

function treeAt(x: number, z: number): boolean {
  const h = heightAt(x, z);
  if (h < 3 || h > 6) return false;
  return hash2i(x, z, 0x5eed) < TREE_DENSITY;
}

// (x, y, z) 是否为实心方块：地表及以下，或属于树木
function isSolid(x: number, y: number, z: number): boolean {
  if (y < 1) return true;
  if (y <= heightAt(x, z)) return true;
  return treeBlocks.has(treeKey(x, y, z));
}

// ============================================================
// 颜色
// ============================================================

function computeColors() {
  latte = document.body.classList.contains("theme-Latte");
  const style = getComputedStyle(document.body);
  const rgb = style.getPropertyValue("--ctp-crust-rgb").trim() || "30,30,46";
  fillColor = "rgb(" + rgb + ")";
}

function lineColor(alpha: number): string {
  return latte ? "rgba(0,0,0," + alpha + ")" : "rgba(255,255,255," + alpha + ")";
}

// ============================================================
// 双曲线天空（离屏缓存，仅在尺寸 / 主题变化时重绘）
// ============================================================

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

function drawHyperbolas(ctx: CanvasRenderingContext2D) {
  ctx.save();

  // 对称中心位于天空中央（窗口高度百分比，见 HYPERBOLA_CENTER_Y_PERCENT）
  ctx.translate(viewW / 2, viewH * HYPERBOLA_CENTER_Y_PERCENT);

  // 主轴方向
  ctx.rotate((-40 * Math.PI) / 180);

  ctx.lineWidth = 0.6;

  const b = 200;

  const curves = [40, 110, 180, 280, 420, 640];

  const range = Math.max(viewW, viewH) * 1.5;

  curves.forEach((a, index) => {
    ctx.strokeStyle = lineColor(0.45 - index * 0.055);

    drawHyperbola(ctx, a, b, range);
  });

  ctx.restore();
}

function getSkyCanvas(): HTMLCanvasElement | null {
  const theme = document.body.className;
  const dpr = window.devicePixelRatio || 1;
  const bw = Math.max(1, Math.round(viewW * dpr));
  const bh = Math.max(1, Math.round(viewH * dpr));

  if (skyCanvas && skyCanvas.width === bw && skyCanvas.height === bh && skyThemeKey === theme) {
    return skyCanvas;
  }

  skyThemeKey = theme;

  const c = document.createElement("canvas");
  c.width = bw;
  c.height = bh;

  const ctx = c.getContext("2d");
  if (ctx) {
    ctx.scale(dpr, dpr);
    drawHyperbolas(ctx);
  }

  skyCanvas = c;
  return c;
}

// ============================================================
// 方块渲染
// ============================================================

function drawBlockFaces(x: number, y: number, z: number) {
  const ctx = gctx;
  if (!ctx) return;

  const dzMin = z - camZ;
  if (dzMin < 0.02) return;

  // 投影八个角点
  const pts: number[] = new Array(16);
  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;

  for (let i = 0; i < 8; i++) {
    const off = CORNER_OFFSETS[i];
    const dz = z + off[2] - camZ;
    const s = focal / dz;
    const sx = halfW + (x + off[0] - CAM_X) * s;
    const sy = halfH - (y + off[1] - camY) * s;
    pts[i * 2] = sx;
    pts[i * 2 + 1] = sy;
    if (sx < minX) minX = sx;
    if (sx > maxX) maxX = sx;
    if (sy < minY) minY = sy;
    if (sy > maxY) maxY = sy;
  }

  if (maxX < 0 || minX > viewW || maxY < 0 || minY > viewH) return;

  // 仅保留“面向摄像机”且“未被相邻方块挡住”的面
  const faces: number[][] = [];
  if (!isSolid(x, y, z - 1)) faces.push(FACE_NZ);
  if (CAM_X < x && !isSolid(x - 1, y, z)) faces.push(FACE_NX);
  if (CAM_X > x + 1 && !isSolid(x + 1, y, z)) faces.push(FACE_PX);
  if (camY > y + 1 && !isSolid(x, y + 1, z)) faces.push(FACE_PY);
  if (camY < y && !isSolid(x, y - 1, z)) faces.push(FACE_NY);

  if (faces.length === 0) return;

  const dzAvg = z + 0.5 - camZ;
  const t = Math.max(0, Math.min(1, (dzAvg - 1) / VIEW_DISTANCE));
  const alpha = WORLD_STROKE_ALPHA * (1 - 0.7 * t);

  ctx.fillStyle = fillColor;
  ctx.strokeStyle = lineColor(alpha);
  ctx.lineWidth = WORLD_LINE_WIDTH;

  for (const face of faces) {
    let bx0 = Infinity;
    let bx1 = -Infinity;
    let by0 = Infinity;
    let by1 = -Infinity;

    for (let i = 0; i < 4; i++) {
      const idx = face[i] * 2;
      const px = pts[idx];
      const py = pts[idx + 1];
      if (px < bx0) bx0 = px;
      if (px > bx1) bx1 = px;
      if (py < by0) by0 = py;
      if (py > by1) by1 = py;
    }

    if (Math.max(bx1 - bx0, by1 - by0) < MIN_FACE_PX) continue;

    ctx.beginPath();
    ctx.moveTo(pts[face[0] * 2], pts[face[0] * 2 + 1]);
    for (let i = 1; i < 4; i++) {
      ctx.lineTo(pts[face[i] * 2], pts[face[i] * 2 + 1]);
    }
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
  }
}

function drawColumn(x: number, z: number) {
  const h = heightAt(x, z);
  const hFront = heightAt(x, z - 1);

  let sideH = hFront;
  if (x >= 1) sideH = Math.min(sideH, heightAt(x - 1, z));
  if (x <= -2) sideH = Math.min(sideH, heightAt(x + 1, z));

  const yStart = Math.max(1, Math.min(h, hFront, sideH));

  for (let y = yStart; y <= h; y++) {
    drawBlockFaces(x, y, z);
  }

  // 树木方块
  if (treeAt(x, z)) {
    const y0 = h + 1;
    for (const [dx, dy, dz] of TREE_SHAPE) {
      drawBlockFaces(x + dx, y0 + dy, z + dz);
    }
  }
}

function drawWorld() {
  const ctx = gctx;
  if (!ctx) return;

  heightCache.clear();
  treeBlocks.clear();

  focal = halfW / Math.tan(((FOV_DEGREES / 2) * Math.PI) / 180);

  const zBase = Math.floor(camZ);
  const zNear = zBase + 1;
  const zFar = zBase + VIEW_DISTANCE;
  const xMin = Math.floor(CAM_X - MAX_HALF_X);
  const xMax = Math.ceil(CAM_X + MAX_HALF_X);

  // 预收集可见区域内的树木方块（供遮挡判断 + 随列渲染）
  for (let z = zBase; z <= zFar; z++) {
    for (let x = xMin; x <= xMax; x++) {
      if (treeAt(x, z)) {
        const y0 = heightAt(x, z) + 1;
        for (const [dx, dy, dz] of TREE_SHAPE) {
          treeBlocks.add(treeKey(x + dx, y0 + dy, z + dz));
        }
      }
    }
  }

  // 地平线以下的地面区域整体铺成背景色，遮住透出的双曲线
  ctx.fillStyle = fillColor;
  ctx.fillRect(0, halfH, viewW, viewH - halfH);

  // 按 z 从远到近、x 从远到近、y 从低到高绘制（画家算法）
  for (let z = zFar; z >= zNear; z--) {
    for (let i = 0; i < xOrder.length; i++) {
      const x = xOrder[i];
      if (x < xMin || x > xMax) continue;
      drawColumn(x, z);
    }
  }

  // 地平线附近的远景雾带，使网格在远处平滑消失
  const fogH = Math.max(10, viewH * 0.045);
  const grad = ctx.createLinearGradient(0, halfH, 0, halfH + fogH);
  grad.addColorStop(0, lineColor(0.12));
  grad.addColorStop(1, lineColor(0));
  ctx.fillStyle = grad;
  ctx.fillRect(0, halfH, viewW, fogH);
}

// ============================================================
// 主循环
// ============================================================

function draw() {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const w = Math.max(1, rect.width);
  const h = Math.max(1, rect.height);
  const dpr = window.devicePixelRatio || 1;
  const bw = Math.round(w * dpr);
  const bh = Math.round(h * dpr);

  if (canvas.width !== bw) canvas.width = bw;
  if (canvas.height !== bh) canvas.height = bh;

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  viewW = w;
  viewH = h;
  halfW = w / 2;
  halfH = h / 2;

  computeColors();

  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, bw, bh);

  const sky = getSkyCanvas();
  if (sky) ctx.drawImage(sky, 0, 0);

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  gctx = ctx;
  drawWorld();
}

function frame(time: number) {
  if (lastTime === 0) lastTime = time;

  const dt = Math.min(0.05, (time - lastTime) / 1000);
  lastTime = time;

  camZ += CAMERA_SPEED * dt;

  draw();

  raf = requestAnimationFrame(frame);
}

onMounted(() => {
  lastTime = 0;
  raf = requestAnimationFrame(frame);

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

onBeforeUnmount(() => {
  cancelAnimationFrame(raf);

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
