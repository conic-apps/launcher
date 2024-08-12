<template>
  <li class="list-item">
    <div :style="`${buttons ? '' : 'justify-content: start;width: 100%;'}${clickAble ? 'pointer-events: all;' : ''}`">
      <div class="icon" :style="logo
          ? `background-image: url(${logo}); ${logoPixelated ? 'image-rendering: pixelated;' : ''}`
          : 'display: none;'
        ">
        <slot name="icon"></slot>
      </div>
      <div>
        <h4>
          {{ title }}
          <div class="sub-title">
            <slot name="subtitle"></slot>
          </div>
        </h4>
        <p>
          {{ description ? description : "" }}
          <slot></slot>
        </p>
      </div>
    </div>
    <div v-if="buttons" class="list-item-buttons">
      <i v-for="(item, index) in buttons" :key="index" class="list-item-button" :class="item"
        @click.stop="$emit(`event-${item}`)"></i>
    </div>
  </li>
</template>

<script setup lang="ts">
defineProps<{
  logo?: string;
  title: string;
  description?: string;
  logoPixelated?: boolean;
  buttons?: string[]; // 图标名称对应点击后触发的事件名称
  clickAble?: boolean;
}>();
</script>

<style lang="less" scoped>
.list-item {
  display: flex;
  padding: 10px 12px;
  border-radius: var(--border-radius-medium);
  transition: all 0.1s cubic-bezier(0, 0.43, 0.25, 1);
  justify-content: space-between;
  border: 1px solid var(--expander-border-color);
  margin-bottom: 6px;
  position: relative;
  // flex-direction: row-reverse;
  overflow: hidden;
  background-color: #ffffff71;
  pointer-events: none;
}

.dialog .list-item {
  background-color: #ffffffb9;
  border: 1px solid #0000002e;
}

.list-item>div {
  display: flex;
  align-items: center;
  overflow: hidden;
}

.list-item>div:first-child {
  width: 100%;
  transition: all 0.1s ease;
}

// .list-item>div:first-child:hover {
//   opacity: 0.6;
// }

.list-item>div:first-child:active {
  opacity: 0.6;
}

.list-item>div:last-child {
  flex-shrink: 0;
  /* overflow-x: hidden; */
  width: fit-content;
  overflow-y: visible;
  justify-content: flex-end;
}

.list-item>div>div:last-child {
  max-width: inherit;
  width: 100%;
}

.list-item h4 {
  font-weight: normal;
  font-size: calc(15px - var(--font-size-error));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  width: fit-content;
  margin-bottom: 3px;
  display: flex;
  align-items: center;
}

.list-item .sub-title {
  color: #0000006f;
  font-size: calc(14px - var(--font-size-error));
  margin-left: 4px;
  display: flex;
}

.list-item p {
  font-size: calc(14px - var(--font-size-error));
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  width: fit-content;
  color: rgba(0, 0, 0, 0.6);
}

.icon {
  flex-shrink: 0;
  width: 32px;
  height: 32px;
  background-size: cover;
  background-position: center;
  margin-left: 2px;
  margin-right: 6px;
  // box-shadow: 0 0 2px #00000088;
  overflow: hidden;
  border-radius: var(--border-radius-medium);
  background-image: url(@/assets/images/Unknown_server.webp);
}

.list-item-button {
  font-family: "fa-pro";
  font-style: normal;
  display: flex;
  align-items: center;
  pointer-events: all;
  transition: all 0.2s ease;
  font-size: calc(15px - var(--font-size-error));
  margin: 0 6px;
  transform: scale3d(1, 1, 500);
}

.list-item-button:hover {
  color: rgba(var(--theme-color), 1);
}

.list-item-button:active {
  transform: scale(0.86);
}
</style>
