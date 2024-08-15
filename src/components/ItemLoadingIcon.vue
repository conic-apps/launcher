<template>
  <svg
    v-if="props.status == 'success'"
    width="16"
    height="16"
    class="success"
    aria-label="completed successfully: "
    viewBox="0 0 16 16"
    version="1.1"
    role="img"
    fill="#3fb950">
    <path
      d="M8 16A8 8 0 1 1 8 0a8 8 0 0 1 0 16Zm3.78-9.72a.751.751 0 0 0-.018-1.042.751.751 0 0 0-1.042-.018L6.75 9.19 5.28 7.72a.751.751 0 0 0-1.042.018.751.751 0 0 0-.018 1.042l2 2a.75.75 0 0 0 1.06 0Z"></path>
  </svg>
  <svg
    v-if="props.status == 'in-progress'"
    width="16"
    height="16"
    fill="none"
    class="in-progress"
    xmlns="http://www.w3.org/2000/svg"
    aria-label="In progress">
    <path
      opacity=".5"
      d="M8 15A7 7 0 108 1a7 7 0 000 14v0z"
      stroke="rgba(255,255,255, 0.9)"
      stroke-width="2.4"></path>
    <path d="M15 8a7 7 0 01-7 7" stroke="rgba(255,255,255,0.9)" stroke-width="2.4"></path></svg
  ><svg
    v-if="props.status == 'pending'"
    class="pending"
    height="16"
    viewBox="0 0 16 16"
    version="1.1"
    width="16"
    aria-hidden="true"
    fill="rgba(255,255,255,0.4)">
    <path
      d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Z"></path></svg
  ><svg
    v-if="props.status == 'error'"
    width="16"
    height="16"
    class="error"
    aria-label="failed: "
    viewBox="0 0 16 16"
    version="1.1"
    role="img"
    fill="#f85149">
    <path
      d="M2.343 13.657A8 8 0 1 1 13.658 2.343 8 8 0 0 1 2.343 13.657ZM6.03 4.97a.751.751 0 0 0-1.042.018.751.751 0 0 0-.018 1.042L6.94 8 4.97 9.97a.749.749 0 0 0 .326 1.275.749.749 0 0 0 .734-.215L8 9.06l1.97 1.97a.749.749 0 0 0 1.275-.326.749.749 0 0 0-.215-.734L9.06 8l1.97-1.97a.749.749 0 0 0-.326-1.275.749.749 0 0 0-.734.215L8 6.94Z"></path>
  </svg>
</template>
<script lang="ts" setup>
const props = defineProps<{
  status: "in-progress" | "pending" | "success" | "error";
}>();
</script>
<style lang="less">
.in-progress {
  animation: rotate-keyframes 1s linear infinite;
}

@keyframes rotate-keyframes {
  100% {
    transform: rotate(360deg);
  }
}
</style>
