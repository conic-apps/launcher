<template>
  <div>
    <expander title="常规" icon="house" :can-swap="false" description="基本扩展设置">
      <toggle-switch-bar name="启用所有扩展"></toggle-switch-bar>
    </expander>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Expander from '@/components/Expander.vue';
import ToggleSwitchBar from '@/components/ToggleSwitchBar.vue';

</script>

<style lang="less" scoped></style>