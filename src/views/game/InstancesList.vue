<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="instances-list">
    <div class="tool-bar"></div>
    <div class="scroll-container" ref="container" @scroll="updatePositions">
      <div class="gap-top"></div>
      <div
        class="card-container"
        :class="{ current: instance.id === instanceStore.currentInstance.id }"
        v-for="instance in instanceStore.instances"
        :key="instance.id">
        <div
          class="instance"
          :class="{ current: instance.id === instanceStore.currentInstance.id }"
          :style="styleMap.get(instance.id)"
          @click="selectInstance(instance)"
          :data-id="instance.id"
          ref="instances">
          <p>{{ instance.config.name }}</p>
          <div class="details">
            <span
              :class="`tag ${instance.config.runtime.mod_loader_type.toLowerCase()}`"
              v-if="instance.config.runtime.mod_loader_type"
              >{{ instance.config.runtime.mod_loader_type }}</span
            >
            <span class="tag vanilla" v-else>Vanilla</span>
            <span class="last-play"><span class="label">上次运行：</span>昨天</span>
          </div>
        </div>
      </div>
      <div class="gap-bottom"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useInstanceStore } from "@/store/instance";
import { Instance } from "@conic/instance";
import { nextTick, onMounted, reactive, ref, useTemplateRef } from "vue";

const instanceStore = useInstanceStore();
const containerRef = useTemplateRef("container");
const items = useTemplateRef<HTMLElement[]>("instances");

const styleMap = reactive(new Map<string, { transform: string }>());

function updatePositions() {
  const container = containerRef.value;
  const elements = items.value;

  if (!container || !elements) return;

  const containerRect = container.getBoundingClientRect();

  const center = containerRect.height / 2;
  const maxOffset = 128;

  const curveRange = containerRect.height;

  for (const element of elements) {
    const rect = element.getBoundingClientRect();

    const y = rect.top - containerRect.top + rect.height / 2;

    const t = (y - center) / curveRange;

    const clamped = Math.max(-1, Math.min(1, t));

    const x = maxOffset * (1 - clamped * clamped);

    styleMap.set(element.dataset.id!, {
      transform: `translateX(${-x}px)`,
    });
  }
}
function scrollToInstance(instanceId: string, smooth: boolean) {
  const container = containerRef.value;
  const elements = items.value;

  if (!container || !elements) return;

  const element = elements.find((el) => el.dataset.id === instanceId);

  if (!element) return;

  const containerRect = container.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();

  const offset =
    elementRect.top + elementRect.height / 2 - (containerRect.top + containerRect.height / 2);

  if (smooth) {
    container.scrollTo({
      top: container.scrollTop + offset,
      behavior: "smooth",
    });
  } else {
    container.scrollTo({
      top: container.scrollTop + offset,
    });
  }
}

async function selectInstance(instance: Instance) {
  scrollToInstance(instance.id, true);
  await nextTick();
  instanceStore.currentInstance = instance;
}

onMounted(async () => {
  await nextTick();
  updatePositions();
  scrollToInstance(instanceStore.currentInstance.id, false);
});
</script>

<style lang="less" scoped>
.instances-list {
  height: 100%;
  width: fit-content;
  margin-left: auto;
  transform: translateX(280px);
  overflow: visible;
  .scroll-container {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    padding-left: 200px;
    .gap-top {
      height: calc(50% - 30px);
    }
    .gap-bottom {
      height: 30%;
    }
  }
  .instance {
    border: 1px solid rgba(var(--ctp-surface1-rgb), 0.8);
    border-left: 16px solid rgba(var(--ctp-surface1-rgb), 0.8);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    padding: 8px 16px;
    border-radius: 8px;
    margin-top: 2px;
    width: 480px;
    height: 60px;
    transition:
      border-left 200ms ease,
      margin 200ms ease;
    p {
      font-size: 16px;
    }
    .details {
      margin-top: 6px;
      .tag {
        font-size: 11px;
        border-radius: 100px;
        padding: 1px 6px;
        font-weight: 500;
      }
      .tag.quilt {
        background: var(--ctp-mauve);
        color: var(--ctp-text-inverse);
      }
      .tag.fabric {
        background: var(--ctp-yellow);
        color: var(--ctp-text-inverse);
      }
      .tag.neoforge {
        background: var(--ctp-peach);
        color: var(--ctp-text-inverse);
      }
      .tag.vanilla {
        background: var(--ctp-green);
        color: var(--ctp-text-inverse);
      }
      .last-play {
        font-size: 11px;
        margin-left: 8px;
        font-weight: 500;
        .label {
          opacity: 0.8;
          font-weight: 300;
        }
      }
    }
  }
  .instance.current {
    border-left: 16px solid rgba(var(--ctp-lavender-rgb), 0.8);
    margin-left: -20px;
  }

  .card-container {
    transition: all 100ms linear;
    margin-top: 0px;
    margin-bottom: 0px;

    &:active {
      transform: scale(0.99);
    }
  }
  .card-container.current {
    transform: scale(1.03);
    margin-top: 4px;
    margin-bottom: 4px;
    pointer-events: none;
  }
}
</style>
