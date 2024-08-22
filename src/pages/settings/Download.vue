<template>
  <keep-alive>
    <div>
      <setting-group>
        <setting-item title="最大连接数" description="限制同时与服务器建立的连接数量" icon="link">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
        <setting-item title="最大下载速度" description="限制下载速度" icon="gauge">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
        <setting-item
          title="镜像服务器"
          :clickAble="true"
          description="从其他地方下载游戏文件"
          icon="server"
          :last="true">
          <i class="chevron-right" style="margin-right: 10px"></i>
        </setting-item>
      </setting-group>
      <setting-group title="代理">
        <setting-item title="使用系统代理" icon="globe">
          <toggle-switch></toggle-switch>
        </setting-item>
      </setting-group>
    </div>
  </keep-alive>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
</script>

<style lang="less" scoped></style>
