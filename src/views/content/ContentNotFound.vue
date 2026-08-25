<template>
  <Transition name="content-placeholder-transition">
    <div class="content-placeholder" v-if="props.show">
      <AppIcon name="about" :size="64"></AppIcon>
      <p class="message">{{ t("content.common.noResult") }}</p>
      <p class="desc" v-if="props.description">{{ props.description }}</p>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const props = defineProps<{
  show: boolean;
  description?: string;
}>();
</script>

<style lang="less" scoped>
.content-placeholder {
  right: 320px;
  width: 380px;
  height: fit-content;
  display: flex;
  align-items: center;
  flex-direction: column;
  text-align: center;
  .message {
    font-size: 16px;
    margin-top: 12px;
  }
  .desc {
    font-size: 12px;
    margin-top: 12px;
  }
}

.content-placeholder-transition-enter-active {
  transition:
    transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1),
    opacity 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.content-placeholder-transition-leave-active {
  transition:
    transform 0.25s cubic-bezier(0.215, 0.61, 0.355, 1),
    opacity 0.25s cubic-bezier(0.215, 0.61, 0.355, 1);
}
.content-placeholder-transition-enter-from,
.content-placeholder-transition-leave-to {
  opacity: 0;
  transform: scale(0.9);
}
</style>
