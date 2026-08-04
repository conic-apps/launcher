<!-- Conic Launcher -->
<!-- Copyright 2022-2026 Broken-Deer and contributors. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="connect-extension-download" data-tauri-drag-region>
    <p>下载 Conic Connect 扩展</p>
    <div class="container">
      <p class="description">
        Conic Connect 扩展基于 Easytier 构建，参考了陶瓦联机的部分实现，可以与其他主流启动器联机
      </p>
      <p class="description">
        此扩展只使用了 Easytier 的 P2P
        连接，联机成功后房间内用户之间将直接连接，不使用第三方服务器转发您的流量，联机体验取决于参与联机者的网络情况。
      </p>
      <p class="description description-important">
        请注意，在多人联机全过程中，你必须严格遵守所在国家与地区的全部法律法规。
      </p>
    </div>
  </div>
</template>

<script setup lang="ts"></script>

<style lang="less" scoped>
.connect-extension-download {
  width: 100%;
  height: 100%;
  border: 1px solid #f00;
}
</style>
