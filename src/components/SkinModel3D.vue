<script setup lang="ts">
import { onMounted, ref, watch } from "vue";

const props = defineProps<{
  skin?: string;
}>();

const canvasRef = ref<HTMLCanvasElement>();

async function loadAndDraw(skin: string) {
  const canvas = canvasRef.value;
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const img = new Image();
  img.src = skin;
  await new Promise<void>((resolve) => {
    img.onload = () => resolve();
    img.onerror = () => resolve();
  });
  if (!img.complete || img.naturalWidth === 0) return;

  const skinScale = img.width / 64;
  const w = canvas.width;
  const h = canvas.height;
  const c = w / 2;
  const zoom = Math.min(w / (16 * skinScale), h / (32 * skinScale)) * 0.85;
  const s = skinScale * zoom;

  function drawModel() {
    if (!ctx) return;
    ctx.clearRect(0, 0, w, h);

    const fw = 8 * s;
    const fh = 8 * s;
    const sw = 8 * s;
    const sh = 12 * s;
    const aw = 4 * s;
    const ah = 12 * s;
    const lw = 4 * s;
    const lh = 12 * s;

    const totalBodyH = fh + sh + lh;
    const startY = (h - totalBodyH) / 2;

    ctx.imageSmoothingEnabled = false;

    ctx.save();
    ctx.translate(c, startY + fh);

    // body
    ctx.save();
    ctx.translate(-sw / 2, 0);
    ctx.drawImage(img, 20 * skinScale, 20 * skinScale, 8 * skinScale, 12 * skinScale, 0, 0, sw, sh);
    ctx.drawImage(img, 20 * skinScale, 36 * skinScale, 8 * skinScale, 12 * skinScale, 0, 0, sw, sh);
    ctx.restore();

    // left arm (character's right arm, screen left)
    ctx.save();
    ctx.translate(-sw / 2, 0);
    ctx.rotate(0.08);
    ctx.drawImage(
      img,
      44 * skinScale,
      20 * skinScale,
      4 * skinScale,
      12 * skinScale,
      -aw,
      0,
      aw,
      ah,
    );
    ctx.drawImage(
      img,
      44 * skinScale,
      36 * skinScale,
      4 * skinScale,
      12 * skinScale,
      -aw,
      0,
      aw,
      ah,
    );
    ctx.restore();

    // right arm (character's left arm, screen right)
    ctx.save();
    ctx.translate(sw / 2, 0);
    ctx.rotate(-0.08);
    ctx.drawImage(img, 36 * skinScale, 52 * skinScale, 4 * skinScale, 12 * skinScale, 0, 0, aw, ah);
    ctx.drawImage(img, 52 * skinScale, 52 * skinScale, 4 * skinScale, 12 * skinScale, 0, 0, aw, ah);
    ctx.restore();

    // left leg (character's right leg, screen left)
    ctx.save();
    ctx.translate(-sw / 2, sh);
    ctx.drawImage(img, 4 * skinScale, 20 * skinScale, 4 * skinScale, 12 * skinScale, 0, 0, lw, lh);
    ctx.drawImage(img, 4 * skinScale, 36 * skinScale, 4 * skinScale, 12 * skinScale, 0, 0, lw, lh);
    ctx.restore();

    // right leg (character's left leg, screen right)
    ctx.save();
    ctx.translate(0, sh);
    ctx.drawImage(img, 20 * skinScale, 52 * skinScale, 4 * skinScale, 12 * skinScale, 0, 0, lw, lh);
    ctx.drawImage(img, 4 * skinScale, 52 * skinScale, 4 * skinScale, 12 * skinScale, 0, 0, lw, lh);
    ctx.restore();

    // head
    ctx.save();
    ctx.translate(-fw / 2, -fh);
    ctx.drawImage(img, 8 * skinScale, 8 * skinScale, 8 * skinScale, 8 * skinScale, 0, 0, fw, fh);
    ctx.drawImage(img, 40 * skinScale, 8 * skinScale, 8 * skinScale, 8 * skinScale, 0, 0, fw, fh);
    ctx.restore();

    ctx.restore();
  }

  drawModel();
}

function startRender() {
  if (props.skin) {
    loadAndDraw(props.skin);
  }
}

onMounted(startRender);

watch(
  () => props.skin,
  (skin) => {
    if (skin) loadAndDraw(skin);
  },
);
</script>

<template>
  <canvas ref="canvasRef" width="80" height="120" class="skin-model"></canvas>
</template>

<style scoped>
.skin-model {
  image-rendering: pixelated;
  width: 80px;
  height: 120px;
}
</style>
