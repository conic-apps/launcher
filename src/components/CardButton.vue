<template>
  <div class="card-button" :style="cardStyle">
    <div class="title">
      <div class="icon"><i :class="icon"></i></div>
      <div>
        <h4 id="text">{{ title }}</h4>
        <p v-if="description" id="text">{{ description }}</p>
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
      <i class="chevron-right"></i>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

const props = withDefaults(
  defineProps<{
    title: string;
    description?: string;
    icon: string;
    margin?: string;
    boxShadow?: boolean;
    padding?: string;
  }>(),
  {
    margin: "",
    boxShadow: true,
    padding: "16,18,16,18",
  },
);
let margin = props.margin.split(",");
let padding = props.padding.split(",");
let cardStyle = `${props.boxShadow ? " box-shadow: 0 0 10px #00000015;" : ""}margin: ${margin[0]}px ${margin[1]}px ${margin[2]}px ${margin[3]}px; padding: ${padding[0]}px ${padding[1]}px ${padding[2]}px ${padding[3]}px;`;
</script>

<style lang="less" scoped>
.card-button {
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid var(--expander-border-color);
  margin: 15px 0 15px 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  transition: all 0.1s ease;
}

.card-button:active {
  opacity: 0.8;
}
// .dialog .card-button {
//   background-color: #ffffffb9;
//   border: 1px solid #0000002e;
// }

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
  border-radius: 10px;
  margin-right: 6px;
  background-color: rgba(255, 255, 255, 0.1);
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

.title h4#text {
  font-weight: normal;
  font-size: 17px;
}

.title p#text {
  font-size: 12.5px;
  color: #ffffffb7;
  opacity: 0.6;
  margin-top: 7px;
}

i.chevron-right {
  transition: transform 0.2s ease;
}

.card-button:hover i.chevron-right {
  transform: translate(4px, 0px);
}
</style>
