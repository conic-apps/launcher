<!-- Amethyst Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<!-- <template>
  <dialog-vue :visible="show" :width="460" :height="480">
    <div class="title">
      <p>管理下载源</p>
      <dialog-button icon="close" @click="$emit('close')"></dialog-button>
    </div>
    <div class="content">
      <div class="tools">
        <button class="command-button">
          添加下载源
        </button>
        <button class="command-button">
          测试速度
        </button>
      </div>
      <TransitionGroup>
        <list-item v-for="(source, index) in sources" :key="index" :title="source.name" :click-able="false"
          :buttons="buttons(source)" @event-circle-info="openBMCLWebsite">
          <template #subtitle>
            <tag v-if="source.type == sourceTypes.built" text="内置" :color="['200', '200', '0']" :background="true"
              :border="true"></tag>
          </template>
          {{ source.description }}
        </list-item>
      </TransitionGroup>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import DialogVue from '@/components/Dialog.vue';
import DialogButton from '@/components/DialogButton.vue';
import ListItem from '@/components/ListItem.vue';
import ButtonBar from '@/components/ButtonBar.vue';
import Tag from '@/components/Tag.vue';

const props = withDefaults(defineProps<{
  show: boolean
}>(), {
  show: false
})

function buttons(source: any) {
  let buttons = []
  if (source.type != sourceTypes.built) {
    buttons.push(...[
      'pro-settings',
      'chevron-up',
      'chevron-down',
    ])
  }
  if (source.bmcl === true) {
    buttons.push('circle-info')
  }
  return buttons
}

function openBMCLWebsite() {
  alert(1)
}

enum sourceTypes {
  built,
}
let sources = ref([
  {
    name: '自定义下载源 #2',
  },
  {
    name: '自定义下载源 #1',
  },
  {
    name: 'BMCLAPI (MCBBS)',
    description: 'BMCLAPI的一个特别镜像',
    type: sourceTypes.built,
    bmcl: true
  },
  {
    name: 'BMCLAPI (传统)',
    description: '由 bangbang93 提供的下载源',
    type: sourceTypes.built,
    bmcl: true
  },
  {
    name: "Amethyst Launcher API",
    description: '包括一些丢失的版本',
    type: sourceTypes.built,
  },
  {
    name: '官方下载源',
    description: '官方提供的下载源',
    type: sourceTypes.built,
  },
])
</script>

<style lang="less" scoped>
.title {
  padding: 14px;
  padding-bottom: 0;
  display: flex;
  justify-content: space-between;
}

.title p {
  font-size: 19px;
}

.content {
  padding: 20px 10px 10px 10px;
}

.tools {
  margin-bottom: 10px;
  display: flex;
  justify-content: space-between;
}

.command-button {
  margin: 0 4px;
}
</style> -->
