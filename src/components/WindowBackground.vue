<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch, useTemplateRef } from "vue";
import gsap from "gsap";
import { useConfigStore } from "@/store/config";

function playIntro() {
  return gsap.timeline().fromTo(
    wrapperRef.value,
    {
      opacity: 0,
    },
    {
      opacity: 1,
      duration: 0.2,
      ease: "power3.out",
    },
  );
}

defineExpose({ playIntro });

// ============================================================
// 可调参数
// ============================================================

// 摄像机前进速度（方块 / 秒）。越大移动越快。
const CAMERA_SPEED = 2;

// 世界渲染视距（方块数）。WebGL + 深度缓冲下可设得比 Canvas 2D 大很多。
const VIEW_DISTANCE = 120;

// 水平视野（度）
const FOV_DEGREES = 60;

// 双曲线背景对称中心的纵向位置（窗口高度百分比，0 = 顶部，1 = 底部）。
// 摄像机水平直视时地平线在窗口中央（50%），天空占上半屏，其中心恰好在 25% 处。
const HYPERBOLA_CENTER_Y_PERCENT = 0.25;

// 天空在地平线（屏幕中线）交界处的淡出范围（占窗口高度的比例，跨地平线上下对称）。
// 让双曲线稍微延伸过地平线后平滑过渡消失，避免生硬截断。
const SKY_FADE_HEIGHT_RATIO = 0.14;

// 树木生成密度：单个方块列出现树木的概率（0~1，越大越密）。
const TREE_DENSITY = 0.0007;

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
  [2, 4, 2],
  [-2, 4, -2],
  [2, 4, -2],
  [-2, 4, 2],
  [2, 3, 2],
  [-2, 3, -2],
  [2, 3, -2],
  [-2, 3, 2],
  [-2, 3, -1],
  [-1, 3, -1],
  [0, 3, -1],
  [1, 3, -1],
  [2, 3, -1],
  [-2, 3, 0],
  [-1, 3, 0],
  [0, 3, 0],
  [1, 3, 0],
  [2, 3, 0],
  [-2, 3, 1],
  [-1, 3, 1],
  [0, 3, 1],
  [1, 3, 1],
  [2, 3, 1],
  [-1, 3, -2],
  [0, 3, -2],
  [1, 3, -2],
  [-1, 3, 2],
  [0, 3, 2],
  [1, 3, 2],
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
  [0, 6, -1],
  [-1, 6, 0],
  [0, 6, 0],
  [1, 6, 0],
  [0, 6, 1],
];

// 世界方块描边不透明度（0~1，越大越明显）。
const WORLD_STROKE_ALPHA = 0.55;

// 方块描边宽度（设备像素）。描边用屏幕空间四边形渲染，亚像素位置也能覆盖整行像素，移动不闪烁。
const EDGE_WIDTH_DEVICE_PX = 2;

// 地形基准高度（地表方块所在 y）
const BASE_HEIGHT = 4;

// 丘陵起伏幅度（方块）
const HILL_AMPLITUDE = 2.2;

// 摄像机高度 = 基准地表高度 + 该值
const EYE_HEIGHT = 10.6;

// 鼠标视差（沿用原实现）
const MAX_OFFSET = 4;
const SCALE = 1.08;

// 远景淡出范围（基于与摄像机的 z 距离）：[FADE_START, FADE_END] 内逐步淡出
const FADE_START = VIEW_DISTANCE * 0.55;
const FADE_END = VIEW_DISTANCE;

// 方块面填充的透明度范围（填充色 = 背景色，用于遮挡，也提供远景雾感）
const FILL_ALPHA_NEAR = 1.0;
const FILL_ALPHA_FAR = 0.15;

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

// 五个可见面各自的角点顺序（均为合法四边形）
const FACE_NZ = [0, 1, 3, 2]; // z 面（z=0 平面，面向摄像机）
const FACE_NX = [0, 2, 6, 4]; // -x 面
const FACE_PX = [1, 5, 7, 3]; // +x 面
const FACE_PY = [2, 3, 7, 6]; // 顶面
const FACE_NY = [0, 1, 5, 4]; // 底面

const MAX_HALF_X = Math.ceil(VIEW_DISTANCE * Math.tan((FOV_DEGREES * Math.PI) / 360)) + 4;

// 摄像机（x = -0.5，使摄像机位于方块中心而非方块边上）
const CAM_X = -0.5;
const CAM_Y = BASE_HEIGHT + EYE_HEIGHT;
let camZ = 0;

// 摄像机所在的方块列（点 (-0.5, 9.6) 落在方块 x=-1、y=9 的包围盒内）。
// 摄像机沿 +z 直线前进会经过该列的所有 z，因此树的任何方块占用 (x=-1, y=9)
// 都会被摄像机穿过，这类树不生成。
const CAM_X_CELL = Math.floor(CAM_X);
const CAM_Y_CELL = Math.floor(CAM_Y);

// ============================================================
// 地形高度环状缓存：窗口随摄像机滑动，只有新进入的一行需要重新计算。
// 0 表示未计算（高度范围为 2..7，不会为 0）。
// ============================================================

const HEIGHT_X_OFF = Math.floor(CAM_X - MAX_HALF_X) - 4;
const HEIGHT_X_SPAN = 2 * MAX_HALF_X + 16;
const HEIGHT_Z_SPAN = VIEW_DISTANCE + 8;
const heights = new Float32Array(HEIGHT_X_SPAN * HEIGHT_Z_SPAN);

// ============================================================
// 可增长的 GPU 顶点 / 索引缓冲（复用，避免每帧分配）
// ============================================================

class FloatVec {
  data: Float32Array;

  len = 0;

  constructor(initial: number) {
    this.data = new Float32Array(initial);
  }

  reset() {
    this.len = 0;
  }

  push(...values: number[]) {
    if (this.len + values.length > this.data.length) {
      let size = this.data.length * 2;
      while (size < this.len + values.length) size *= 2;
      const next = new Float32Array(size);
      next.set(this.data);
      this.data = next;
    }
    for (const v of values) {
      this.data[this.len++] = v;
    }
  }

  // 热路径专用：避免 push(...) 的 rest 数组分配
  push4(a: number, b: number, c: number, d: number) {
    if (this.len + 4 > this.data.length) {
      let size = this.data.length * 2;
      while (size < this.len + 4) size *= 2;
      const next = new Float32Array(size);
      next.set(this.data);
      this.data = next;
    }
    const dta = this.data;
    const l = this.len;
    dta[l] = a;
    dta[l + 1] = b;
    dta[l + 2] = c;
    dta[l + 3] = d;
    this.len = l + 4;
  }

  push8(a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) {
    if (this.len + 8 > this.data.length) {
      let size = this.data.length * 2;
      while (size < this.len + 8) size *= 2;
      const next = new Float32Array(size);
      next.set(this.data);
      this.data = next;
    }
    const dta = this.data;
    const l = this.len;
    dta[l] = a;
    dta[l + 1] = b;
    dta[l + 2] = c;
    dta[l + 3] = d;
    dta[l + 4] = e;
    dta[l + 5] = f;
    dta[l + 6] = g;
    dta[l + 7] = h;
    this.len = l + 8;
  }
}

class UintVec {
  data: Uint32Array;

  len = 0;

  constructor(initial: number) {
    this.data = new Uint32Array(initial);
  }

  reset() {
    this.len = 0;
  }

  push(...values: number[]) {
    if (this.len + values.length > this.data.length) {
      let size = this.data.length * 2;
      while (size < this.len + values.length) size *= 2;
      const next = new Uint32Array(size);
      next.set(this.data);
      this.data = next;
    }
    for (const v of values) {
      this.data[this.len++] = v;
    }
  }

  push6(a: number, b: number, c: number, d: number, e: number, f: number) {
    if (this.len + 6 > this.data.length) {
      let size = this.data.length * 2;
      while (size < this.len + 6) size *= 2;
      const next = new Uint32Array(size);
      next.set(this.data);
      this.data = next;
    }
    const dta = this.data;
    const l = this.len;
    dta[l] = a;
    dta[l + 1] = b;
    dta[l + 2] = c;
    dta[l + 3] = d;
    dta[l + 4] = e;
    dta[l + 5] = f;
    this.len = l + 6;
  }
}

// 填充面顶点（xyz + alpha，4 顶点/面）与索引；描边顶点
// （pos + mix + 另一端点 + 边向符号，每边 6 顶点 / 2 三角形，8 float/顶点）
const fillVerts = new FloatVec(2_000_000);
const fillIdx = new UintVec(600_000);
const edgeVerts = new FloatVec(3_000_000);

// 画布 / 几何状态
const skyCanvasRef = useTemplateRef("skyCanvasRef");
const glCanvasRef = useTemplateRef("glCanvasRef");
const wrapperRef = useTemplateRef("wrapperRef");

let moveX: ((value: number) => void) | undefined;
let moveY: ((value: number) => void) | undefined;

let viewW = 0;
let viewH = 0;

// 当前可见区域内树木占用的方块集合（key 为数值编码）
const treeBlocks = new Set<number>();

// 双曲线天空离屏缓存
let skyCanvas: HTMLCanvasElement | null = null;
let skyThemeKey = "";

let latte = false;

// WebGL 资源
let gl: WebGL2RenderingContext | null = null;
let fillProg: WebGLProgram | null = null;
let edgeProg: WebGLProgram | null = null;
let fillVbo: WebGLBuffer | null = null;
let fillIbo: WebGLBuffer | null = null;
let edgeVbo: WebGLBuffer | null = null;

const fillU: Record<string, WebGLUniformLocation | null> = {};
const edgeU: Record<string, WebGLUniformLocation | null> = {};

const bgColorVec = new Float32Array([30 / 255, 30 / 255, 46 / 255, 1]);
const edgeColorVec = new Float32Array([1, 1, 1, 1]);
let lastTheme = "";

let lastBuiltFloor = NaN;

let raf = 0;
let lastTime = 0;

// 立体背景设置（外观与动效）
const config = useConfigStore();

// 摄像机静止（关闭摄像机移动）时，用于在窗口尺寸 / 主题变化时补一帧渲染
let resizeObserver: ResizeObserver | undefined;
let themeObserver: MutationObserver | undefined;

function onMouseMove(event: MouseEvent) {
  // 关闭“背景图片视差”后不再随鼠标移动
  if (!config.appearance.background_parallax) return;

  const nx = (event.clientX - window.innerWidth / 2) / (window.innerWidth / 2);
  const ny = (event.clientY - window.innerHeight / 2) / (window.innerHeight / 2);

  moveX?.(nx * MAX_OFFSET);
  moveY?.(ny * MAX_OFFSET);
}

function onMouseLeave() {
  if (!config.appearance.background_parallax) return;

  moveX?.(0);
  moveY?.(0);
}

function onContextLost(event: Event) {
  event.preventDefault();
}

function onContextRestored() {
  initGL();
  buildGeometry();
  // 静止模式下不依赖主循环，上下文恢复后需要立即补渲染
  if (!config.appearance.background_camera_move) {
    draw();
  }
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
  const xi = x - HEIGHT_X_OFF;
  if (xi < 0 || xi >= HEIGHT_X_SPAN) return terrainHeight(x, z);
  const zm = ((z % HEIGHT_Z_SPAN) + HEIGHT_Z_SPAN) % HEIGHT_Z_SPAN;
  const idx = xi + zm * HEIGHT_X_SPAN;
  let v = heights[idx];
  if (v === 0) {
    v = terrainHeight(x, z);
    heights[idx] = v;
  }
  return v;
}

// 摄像机前进到新的一行时，清掉新进入行（zFar）的缓存槽
function invalidateHeightRow(zWorld: number) {
  const zm = ((zWorld % HEIGHT_Z_SPAN) + HEIGHT_Z_SPAN) % HEIGHT_Z_SPAN;
  const start = zm * HEIGHT_X_SPAN;
  heights.fill(0, start, start + HEIGHT_X_SPAN);
}

function treeKey(x: number, y: number, z: number): number {
  return x + 256 + y * 1024 + z * 32768;
}

function treeAt(x: number, z: number): boolean {
  const h = heightAt(x, z);
  if (h < 3 || h > 6) return false;
  if (!(hash2i(x, z, 0x5eed) < TREE_DENSITY)) return false;
  return !treeHitsCamera(x, h);
}

// 树的任何方块（树干 + 树叶）是否占用了摄像机穿行的方块列
function treeHitsCamera(x: number, h: number): boolean {
  const y0 = h + 1;
  for (const [dx, dy] of TREE_SHAPE) {
    if (x + dx === CAM_X_CELL && y0 + dy === CAM_Y_CELL) return true;
  }
  return false;
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
  const theme = document.body.className;
  if (theme === lastTheme) return;
  lastTheme = theme;

  latte = document.body.classList.contains("theme-Latte");
  const style = getComputedStyle(document.body);
  const rgb = style.getPropertyValue("--ctp-crust-rgb").trim() || "30,30,46";
  const parts = rgb.split(",");
  bgColorVec[0] = (parseInt(parts[0], 10) || 0) / 255;
  bgColorVec[1] = (parseInt(parts[1], 10) || 0) / 255;
  bgColorVec[2] = (parseInt(parts[2], 10) || 0) / 255;
  const c = latte ? 0 : 255;
  edgeColorVec[0] = c / 255;
  edgeColorVec[1] = c / 255;
  edgeColorVec[2] = c / 255;
}

function lineColor(alpha: number): string {
  return latte ? "rgba(0,0,0," + alpha + ")" : "rgba(255,255,255," + alpha + ")";
}

// ============================================================
// 双曲线天空（Canvas 2D 离屏缓存，仅在尺寸 / 主题变化时重绘）
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
// WebGL 初始化
// ============================================================

const FILL_VS = `#version 300 es
layout(location = 0) in vec3 aPos;
layout(location = 1) in float aAlpha;
uniform float uCamX;
uniform float uCamY;
uniform float uCamZ;
uniform float uFocal;
uniform float uHalfW;
uniform float uHalfH;
uniform float uNearZ;
uniform float uFarZ;
out float vAlpha;
void main() {
  float dz = aPos.z - uCamZ;
  float s = uFocal / dz;
  float ndcX = (aPos.x - uCamX) * s / uHalfW;
  float ndcY = (aPos.y - uCamY) * s / uHalfH;
  float ndcZ = clamp((dz - uNearZ) / (uFarZ - uNearZ) * 2.0 - 1.0, -1.0, 1.0);
  gl_Position = vec4(ndcX, ndcY, ndcZ, 1.0);
  vAlpha = aAlpha;
}`;

const FILL_FS = `#version 300 es
precision highp float;
in float vAlpha;
uniform vec4 uBgColor;
out vec4 outColor;
void main() {
  outColor = vec4(uBgColor.rgb * vAlpha, vAlpha);
}`;

const EDGE_VS = `#version 300 es
layout(location = 0) in vec3 aPos;
layout(location = 1) in float aMix;
layout(location = 2) in vec3 aOther;
layout(location = 3) in float aSide;
uniform float uCamX;
uniform float uCamY;
uniform float uCamZ;
uniform float uFocal;
uniform float uHalfW;
uniform float uHalfH;
uniform float uNearZ;
uniform float uFarZ;
uniform float uEdgeWidth;
out float vMix;
void main() {
  // 把线段在屏幕空间（像素单位）沿垂直方向展开成固定宽度的四边形，
  // 避免原生 LINES 在亚像素位置被光栅化丢弃导致移动时闪烁。
  float dz0 = aPos.z - uCamZ;
  float dz1 = aOther.z - uCamZ;
  float s0 = uFocal / dz0;
  float s1 = uFocal / dz1;
  vec2 p0 = vec2((aPos.x - uCamX) * s0, (aPos.y - uCamY) * s0);
  vec2 p1 = vec2((aOther.x - uCamX) * s1, (aOther.y - uCamY) * s1);
  vec2 d = p1 - p0;
  vec2 n = vec2(-d.y, d.x);
  float len = length(n);
  vec2 off = len > 1e-5 ? (n / len) * (uEdgeWidth * 0.5) : vec2(0.0);
  vec2 p = p0 + off * aSide;
  float ndcX = p.x / uHalfW;
  float ndcY = p.y / uHalfH;
  float ndcZ = clamp((dz0 - uNearZ) / (uFarZ - uNearZ) * 2.0 - 1.0, -1.0, 1.0);
  gl_Position = vec4(ndcX, ndcY, ndcZ, 1.0);
  vMix = aMix;
}`;

const EDGE_FS = `#version 300 es
precision highp float;
in float vMix;
uniform vec4 uBgColor;
uniform vec4 uEdgeColor;
out vec4 outColor;
void main() {
  vec3 c = mix(uBgColor.rgb, uEdgeColor.rgb, vMix);
  outColor = vec4(c, 1.0);
}`;

function createProgram(
  g: WebGL2RenderingContext,
  vsSrc: string,
  fsSrc: string,
): WebGLProgram | null {
  const vs = g.createShader(g.VERTEX_SHADER);
  const fs = g.createShader(g.FRAGMENT_SHADER);
  if (!vs || !fs) return null;

  g.shaderSource(vs, vsSrc);
  g.compileShader(vs);
  if (!g.getShaderParameter(vs, g.COMPILE_STATUS)) {
    g.deleteShader(vs);
    return null;
  }

  g.shaderSource(fs, fsSrc);
  g.compileShader(fs);
  if (!g.getShaderParameter(fs, g.COMPILE_STATUS)) {
    g.deleteShader(vs);
    g.deleteShader(fs);
    return null;
  }

  const prog = g.createProgram();
  if (!prog) return null;

  g.attachShader(prog, vs);
  g.attachShader(prog, fs);
  g.linkProgram(prog);
  g.deleteShader(vs);
  g.deleteShader(fs);

  if (!g.getProgramParameter(prog, g.LINK_STATUS)) {
    g.deleteProgram(prog);
    return null;
  }

  return prog;
}

function getUniforms(g: WebGL2RenderingContext, prog: WebGLProgram, names: readonly string[]) {
  const u: Record<string, WebGLUniformLocation | null> = {};
  for (const n of names) {
    u[n] = g.getUniformLocation(prog, n);
  }
  return u;
}

function initGL(): boolean {
  const canvas = glCanvasRef.value;
  if (!canvas) return false;

  const g = canvas.getContext("webgl2", {
    alpha: true,
    depth: true,
    antialias: false,
    premultipliedAlpha: true,
  });
  if (!g) return false;

  gl = g;

  fillProg = createProgram(g, FILL_VS, FILL_FS);
  edgeProg = createProgram(g, EDGE_VS, EDGE_FS);
  if (!fillProg || !edgeProg) return false;

  const names = ["uCamX", "uCamY", "uCamZ", "uFocal", "uHalfW", "uHalfH", "uNearZ", "uFarZ"];
  Object.assign(fillU, getUniforms(g, fillProg, names));
  Object.assign(fillU, getUniforms(g, fillProg, ["uBgColor"]));
  Object.assign(edgeU, getUniforms(g, edgeProg, names));
  Object.assign(edgeU, getUniforms(g, edgeProg, ["uBgColor", "uEdgeColor", "uEdgeWidth"]));

  fillVbo = g.createBuffer();
  fillIbo = g.createBuffer();
  edgeVbo = g.createBuffer();

  g.disable(g.CULL_FACE);

  return true;
}

// ============================================================
// 世界几何构建（仅在摄像机每前进一整格时重建）
// ============================================================

function emitFace(x: number, y: number, z: number, face: readonly number[]) {
  if (z - camZ < 0.05) return;

  const dz = z + 0.5 - camZ;
  const fade = Math.max(0, Math.min(1, (dz - FADE_START) / (FADE_END - FADE_START)));
  if (fade >= 1) return;

  const fillAlpha = FILL_ALPHA_NEAR + (FILL_ALPHA_FAR - FILL_ALPHA_NEAR) * fade;
  const edgeMix = WORLD_STROKE_ALPHA * (1 - fade);

  const base = fillVerts.len / 4;
  for (let i = 0; i < 4; i++) {
    const c = CORNER_OFFSETS[face[i]];
    fillVerts.push4(x + c[0], y + c[1], z + c[2], fillAlpha);
  }
  fillIdx.push6(base, base + 1, base + 2, base, base + 2, base + 3);

  for (let i = 0; i < 4; i++) {
    const c0 = CORNER_OFFSETS[face[i]];
    const c1 = CORNER_OFFSETS[face[(i + 1) % 4]];
    const ax = x + c0[0];
    const ay = y + c0[1];
    const az = z + c0[2];
    const bx = x + c1[0];
    const by = y + c1[1];
    const bz = z + c1[2];
    // 每边 6 顶点组成两个屏幕空间三角形（顶点着色器负责展开）。
    // 顶点布局：本端点 (xyz) + mix + 另一端点 (xyz) + 边向符号。
    edgeVerts.push8(ax, ay, az, edgeMix, bx, by, bz, 1);
    edgeVerts.push8(ax, ay, az, edgeMix, bx, by, bz, -1);
    edgeVerts.push8(bx, by, bz, edgeMix, ax, ay, az, 1);
    edgeVerts.push8(ax, ay, az, edgeMix, bx, by, bz, -1);
    edgeVerts.push8(bx, by, bz, edgeMix, ax, ay, az, 1);
    edgeVerts.push8(bx, by, bz, edgeMix, ax, ay, az, -1);
  }
}

function emitBlock(x: number, y: number, z: number) {
  // 仅保留“面向摄像机”且“未被相邻方块挡住”的面
  if (!isSolid(x, y, z - 1)) emitFace(x, y, z, FACE_NZ);
  if (CAM_X < x && !isSolid(x - 1, y, z)) emitFace(x, y, z, FACE_NX);
  if (CAM_X > x + 1 && !isSolid(x + 1, y, z)) emitFace(x, y, z, FACE_PX);
  if (CAM_Y > y + 1 && !isSolid(x, y + 1, z)) emitFace(x, y, z, FACE_PY);
  if (CAM_Y < y && !isSolid(x, y - 1, z)) emitFace(x, y, z, FACE_NY);
}

function buildColumn(x: number, z: number) {
  const h = heightAt(x, z);
  const hFront = heightAt(x, z - 1);

  let sideH = hFront;
  if (x >= 1) sideH = Math.min(sideH, heightAt(x - 1, z));
  if (x <= -2) sideH = Math.min(sideH, heightAt(x + 1, z));

  const yStart = Math.max(1, Math.min(h, hFront, sideH));

  for (let y = yStart; y <= h; y++) {
    emitBlock(x, y, z);
  }

  // 树木方块
  if (treeAt(x, z)) {
    const y0 = h + 1;
    for (const [dx, dy, dz] of TREE_SHAPE) {
      emitBlock(x + dx, y0 + dy, z + dz);
    }
  }
}

function buildGeometry() {
  const g = gl;
  if (!g || !fillVbo || !fillIbo || !edgeVbo) return;

  const zBase = Math.floor(camZ);
  const zNear = zBase + 1;
  const zFar = zBase + VIEW_DISTANCE;
  const xMin = Math.floor(CAM_X - MAX_HALF_X);
  const xMax = Math.ceil(CAM_X + MAX_HALF_X);

  invalidateHeightRow(zFar);
  treeBlocks.clear();
  fillVerts.reset();
  fillIdx.reset();
  edgeVerts.reset();

  // 地平线以下的地面基底：在远平面上铺一块背景色四边形，
  // 遮住下方透出的双曲线天空（等价于原 Canvas 版的整块下移 fillRect）。
  const canvas = glCanvasRef.value;
  const rect = canvas?.getBoundingClientRect();
  const aspect = rect && rect.width > 0 ? rect.height / rect.width : 9 / 16;
  const tanHalf = Math.tan(((FOV_DEGREES / 2) * Math.PI) / 180);
  const xr = FADE_END * tanHalf;
  const yr = FADE_END * aspect * tanHalf;
  const wz = camZ + FADE_END;
  fillVerts.push4(CAM_X - xr, CAM_Y, wz, 1);
  fillVerts.push4(CAM_X + xr, CAM_Y, wz, 1);
  fillVerts.push4(CAM_X - xr, CAM_Y - yr, wz, 1);
  fillVerts.push4(CAM_X + xr, CAM_Y - yr, wz, 1);
  fillIdx.push6(0, 1, 2, 1, 3, 2);

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

  for (let z = zFar; z >= zNear; z--) {
    for (let x = xMin; x <= xMax; x++) {
      buildColumn(x, z);
    }
  }

  g.bindBuffer(g.ARRAY_BUFFER, fillVbo);
  g.bufferData(g.ARRAY_BUFFER, fillVerts.data.subarray(0, fillVerts.len), g.DYNAMIC_DRAW);

  g.bindBuffer(g.ELEMENT_ARRAY_BUFFER, fillIbo);
  g.bufferData(g.ELEMENT_ARRAY_BUFFER, fillIdx.data.subarray(0, fillIdx.len), g.DYNAMIC_DRAW);

  g.bindBuffer(g.ARRAY_BUFFER, edgeVbo);
  g.bufferData(g.ARRAY_BUFFER, edgeVerts.data.subarray(0, edgeVerts.len), g.DYNAMIC_DRAW);
}

// ============================================================
// 绘制
// ============================================================

function setCommonUniforms(
  u: Record<string, WebGLUniformLocation | null>,
  halfW: number,
  halfH: number,
  focal: number,
) {
  const g = gl;
  if (!g) return;
  g.uniform1f(u["uCamX"], CAM_X);
  g.uniform1f(u["uCamY"], CAM_Y);
  g.uniform1f(u["uCamZ"], camZ);
  g.uniform1f(u["uFocal"], focal);
  g.uniform1f(u["uHalfW"], halfW);
  g.uniform1f(u["uHalfH"], halfH);
  g.uniform1f(u["uNearZ"], 0.2);
  g.uniform1f(u["uFarZ"], FADE_END);
}

function draw() {
  const g = gl;
  const canvas = glCanvasRef.value;
  const skyC = skyCanvasRef.value;
  if (!g || !canvas || !skyC) return;

  const rect = canvas.getBoundingClientRect();
  const w = Math.max(1, rect.width);
  const h = Math.max(1, rect.height);
  const dpr = window.devicePixelRatio || 1;
  const bw = Math.round(w * dpr);
  const bh = Math.round(h * dpr);

  if (canvas.width !== bw) canvas.width = bw;
  if (canvas.height !== bh) canvas.height = bh;
  if (skyC.width !== bw) skyC.width = bw;
  if (skyC.height !== bh) skyC.height = bh;

  viewW = w;
  viewH = h;

  computeColors();

  // 天空（Canvas 2D，透明，让下方层显示）。
  // 双曲线会向下延伸越过地平线，而世界画布本身只有 30% 不透明度，
  // 所以用 destination-out 渐变把地平线以下的天空按透明度逐步擦除：
  // 上半部分保留，越过地平线一点后平滑淡出，避免生硬截断。
  const sctx = skyC.getContext("2d");
  if (sctx) {
    sctx.setTransform(1, 0, 0, 1, 0, 0);
    sctx.clearRect(0, 0, bw, bh);
    const sky = getSkyCanvas();
    if (sky) {
      sctx.drawImage(sky, 0, 0);
      const fadeBand = bh * SKY_FADE_HEIGHT_RATIO;
      const fadeTop = bh / 1.65 - fadeBand / 2;
      const fadeBottom = bh / 1.65 + fadeBand / 2;
      sctx.save();
      sctx.globalCompositeOperation = "destination-out";
      // 渐变从 fadeTop 透明到 fadeBottom 全不透明；fillRect 一直铺到底部，
      // 渐变在此之后保持最后一档（alpha=1），确保淡出带以下完全擦除。
      const grad = sctx.createLinearGradient(0, fadeTop, 0, fadeBottom);
      grad.addColorStop(0, "rgba(0,0,0,0)");
      grad.addColorStop(1, "rgba(0,0,0,1)");
      sctx.fillStyle = grad;
      sctx.fillRect(0, fadeTop, bw, bh - fadeTop);
      sctx.restore();
    }
  }

  // 世界（WebGL2，深度缓冲保证遮挡，无需排序）
  g.viewport(0, 0, bw, bh);
  // depthMask 必须先恢复为 true，否则上一帧描边通道留下的
  // depthMask(false) 会让本帧清不掉深度缓冲，地平线下的地面基底
  // 四边形会因残留深度（<1.0）在 LEQUAL 测试中失败而透出天空。
  g.depthMask(true);
  g.clearColor(0, 0, 0, 0);
  g.clear(g.COLOR_BUFFER_BIT | g.DEPTH_BUFFER_BIT);

  const halfW = bw / 2;
  const halfH = bh / 2;
  const focal = halfW / Math.tan(((FOV_DEGREES / 2) * Math.PI) / 180);

  // 填充面（背景色，预乘 alpha 混合，写入深度）
  // LEQUAL：地平线以下的地面基底四边形位于远平面（深度恰为 1.0），
  // LESS 会让它深度测试失败而无法绘制。
  g.useProgram(fillProg);
  setCommonUniforms(fillU, halfW, halfH, focal);
  g.uniform4fv(fillU["uBgColor"], bgColorVec);
  g.enable(g.BLEND);
  g.blendFunc(g.ONE, g.ONE_MINUS_SRC_ALPHA);
  g.enable(g.DEPTH_TEST);
  g.depthFunc(g.LEQUAL);
  g.depthMask(true);
  g.bindBuffer(g.ARRAY_BUFFER, fillVbo);
  g.enableVertexAttribArray(0);
  g.vertexAttribPointer(0, 3, g.FLOAT, false, 16, 0);
  g.enableVertexAttribArray(1);
  g.vertexAttribPointer(1, 1, g.FLOAT, false, 16, 12);
  g.bindBuffer(g.ELEMENT_ARRAY_BUFFER, fillIbo);
  g.drawElements(g.TRIANGLES, fillIdx.len, g.UNSIGNED_INT, 0);

  // 描边（颜色已按距离向背景色混合，不混合、深度等于或小于即绘制；
  // 屏幕空间四边形，宽度 EDGE_WIDTH_DEVICE_PX 设备像素）
  g.useProgram(edgeProg);
  setCommonUniforms(edgeU, halfW, halfH, focal);
  g.uniform4fv(edgeU["uBgColor"], bgColorVec);
  g.uniform4fv(edgeU["uEdgeColor"], edgeColorVec);
  g.uniform1f(edgeU["uEdgeWidth"], EDGE_WIDTH_DEVICE_PX);
  g.disable(g.BLEND);
  g.depthFunc(g.LEQUAL);
  g.depthMask(false);
  g.bindBuffer(g.ARRAY_BUFFER, edgeVbo);
  g.enableVertexAttribArray(0);
  g.vertexAttribPointer(0, 3, g.FLOAT, false, 32, 0);
  g.enableVertexAttribArray(1);
  g.vertexAttribPointer(1, 1, g.FLOAT, false, 32, 12);
  g.enableVertexAttribArray(2);
  g.vertexAttribPointer(2, 3, g.FLOAT, false, 32, 16);
  g.enableVertexAttribArray(3);
  g.vertexAttribPointer(3, 1, g.FLOAT, false, 32, 28);
  g.drawArrays(g.TRIANGLES, 0, edgeVerts.len / 8);
}

// ============================================================
// 主循环
// ============================================================

function frame(time: number) {
  if (lastTime === 0) lastTime = time;

  const dt = Math.min(0.05, (time - lastTime) / 1000);
  lastTime = time;

  // 关闭“立体背景摄像机移动”后摄像机停止，主循环不再继续调度
  if (!config.appearance.background_camera_move) {
    return;
  }

  camZ += CAMERA_SPEED * dt;

  // 几何只在摄像机每前进一整格时重建，其余帧仅更新 uniform
  const floor = Math.floor(camZ);
  if (floor !== lastBuiltFloor) {
    lastBuiltFloor = floor;
    buildGeometry();
  }

  draw();

  raf = requestAnimationFrame(frame);
}

onMounted(() => {
  lastTime = 0;
  lastBuiltFloor = NaN;

  initGL();
  buildGeometry();

  const canvas = glCanvasRef.value;
  if (canvas) {
    canvas.addEventListener("webglcontextlost", onContextLost);
    canvas.addEventListener("webglcontextrestored", onContextRestored);

    // 静止模式下窗口尺寸变化时重建并重绘（其余时间不运行主循环，零开销）
    resizeObserver = new ResizeObserver(() => {
      if (!config.appearance.background_camera_move) {
        buildGeometry();
        draw();
      }
    });
    resizeObserver.observe(canvas);
  }

  // 静止模式下主题变化（调色板 / 高对比 / 跟随系统）时重绘一次
  themeObserver = new MutationObserver(() => {
    if (!config.appearance.background_camera_move) {
      buildGeometry();
      draw();
    }
  });
  themeObserver.observe(document.body, { attributes: true, attributeFilter: ["class"] });

  if (config.appearance.background_camera_move) {
    raf = requestAnimationFrame(frame);
  } else {
    // 摄像机静止：渲染出方块后不再更新
    draw();
  }

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

// 设置变化时实时切换主循环
watch(
  () => config.appearance.background_camera_move,
  (move) => {
    if (move) {
      lastTime = 0;
      raf = requestAnimationFrame(frame);
    } else {
      cancelAnimationFrame(raf);
      raf = 0;
      draw();
    }
  },
);

// 关闭视差时把背景归位，避免停留在鼠标最后的位置
watch(
  () => config.appearance.background_parallax,
  (parallax) => {
    if (!parallax) {
      moveX?.(0);
      moveY?.(0);
    }
  },
);

onBeforeUnmount(() => {
  cancelAnimationFrame(raf);
  resizeObserver?.disconnect();
  themeObserver?.disconnect();

  window.removeEventListener("mousemove", onMouseMove);
  document.removeEventListener("mouseleave", onMouseLeave);

  const canvas = glCanvasRef.value;
  canvas?.removeEventListener("webglcontextlost", onContextLost);
  canvas?.removeEventListener("webglcontextrestored", onContextRestored);

  if (wrapperRef.value) {
    gsap.killTweensOf(wrapperRef.value);
  }
});
</script>

<template>
  <div ref="wrapperRef" class="background-wrapper">
    <canvas ref="skyCanvasRef" class="background sky" />
    <canvas ref="glCanvasRef" class="background world" />
  </div>
</template>

<style scoped>
.background-wrapper {
  width: 100%;
  height: 100%;
  .background {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
  }
  .sky {
    opacity: 0.3;
  }
  .world {
    opacity: 0.3;
  }
}
</style>
