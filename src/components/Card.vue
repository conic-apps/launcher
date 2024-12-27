<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="card" :style="cardStyle">
    <div class="title">
      <div class="icon" :style="iconStyle"><i :class="icon"></i></div>
      <div>
        <h4 id="text" :style="props.titleFontSize ? `font-size: ${props.titleFontSize}` : ''">
          {{ title }}
        </h4>
        <p
          v-if="description"
          id="text"
          :style="props.descriptionFontSize ? `font-size: ${props.descriptionFontSize}` : ''">
          {{ description }}
        </p>
      </div>
    </div>
    <div
      style="
        width: 36px;
        height: 36px;
        display: flex;
        align-items: center;
        justify-content: center;
      ">
      <!-- <slot></slot>  // todo: controll buttons  -->
    </div>
  </div>
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon?: string;
    margin?: string;
    boxShadow?: boolean;
    padding?: string;
    iconSize?: string;
    iconBackground?: boolean;
    titleFontSize?: string;
    descriptionFontSize?: string;
  }>(),
  {
    margin: "",
    boxShadow: true,
    padding: "16,18,16,18",
    icon: "",
    iconBackground: true,
  },
);
let margin = props.margin.split(",");
let padding = props.padding.split(",");
let cardStyle = `${props.boxShadow ? " box-shadow: 0 0 10px #00000015;" : ""}margin: ${margin[0]}px ${margin[1]}px ${margin[2]}px ${margin[3]}px; padding: ${padding[0]}px ${padding[1]}px ${padding[2]}px ${padding[3]}px;`;
let iconSize = props.iconSize?.split(",");
let iconStyle =
  (iconSize ? `width: ${iconSize[0]}px; height: ${iconSize[1]}px;` : "") +
  (props.iconBackground ? "" : "background: none;");
</script>

<style lang="less" scoped>
.card {
  margin: 15px 0 15px 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
  border-radius: var(--card-border-radius);
  background: var(--card-background);
  border: var(--card-border);
  margin: 15px 0 15px 0;
  transition: all 0.1s ease;
}

.card:active {
  opacity: 0.8;
}

.title {
  display: flex;
}

.icon {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  width: 48px;
  height: 48px;
  border-radius: var(--card-border-radius);
  margin: auto 6px auto auto;
  background: var(--card-icon-background);
  background-position: center;
  background-size: contain;
  background-repeat: none;
  image-rendering: pixelated;
}

.icon i {
  font-family: "fa-pro";
  font-style: normal;
  font-size: 20px;
  font-weight: 500;
}

.title > div {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.title h4 {
  font-weight: normal;
  font-size: 14.5px;
  margin-bottom: 0.15em;
  margin-top: 0.08em;
}

.title p {
  font-size: 12.5px;
  color: rgba(var(--default-text-color) 0.7);
  opacity: 0.6;
  margin-top: 0px;
}

i.chevron-right {
  transition: transform 0.2s ease;
}
</style>
