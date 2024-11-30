<template>
  <div class="settings">
    <setting-group>
      <setting-item
        title="Instance Name"
        description="The name of this game instance."
        icon="signature">
        <TextInputBox width="300px" v-model="instanceName"></TextInputBox>
      </setting-item>
      <setting-item title="Icon" description="The name of this game instance." icon="icon">
        <img width="32px" height="32px" src="@/assets/images/Grass_Block.webp" alt="" />
        <i class="chevron-right" style="margin-right: 10px; margin-left: 8px"></i>
      </setting-item>
    </setting-group>
    <setting-group :title="$t('settings.game.launchOptions')">
      <setting-item
        :title="$t('settings.game.launcherName')"
        :description="$t('settings.game.launcherNameDesc')"
        icon="signature">
        <TextInputBox width="300px" v-model="config.launch.launcher_name"></TextInputBox>
      </setting-item>
      <!-- TODO:<setting-item title="服务器地址" description="启动后自动加入服务器" icon="server"> -->
      <!--   <TextInputBox -->
      <!--     width="240px" -->
      <!--     v-model="config.launch.server!.ip" -->
      <!--     style="display: inline-block; margin-right: 16px" -->
      <!--     placeholder="IP 或域名"></TextInputBox> -->
      <!--   <TextInputBox -->
      <!--     width="100px" -->
      <!--     v-model="config.launch.server!.port" -->
      <!--     placeholder="端口" -->
      <!--     style="display: inline-block"></TextInputBox> -->
      <!-- </setting-item> -->
      <setting-item
        :title="$t('settings.game.worldName')"
        :description="$t('settings.game.worldNameDesc')"
        icon="floppy-disk">
        <TextInputBox
          width="300px"
          :placeholder="$t('settings.game.worldNamePlaceholder')"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.game.fullscreen')"
        :description="$t('settings.game.fullscreenDesc')"
        icon="window-maximize">
        <ToggleSwitch v-model="config.launch.fullscreen"></ToggleSwitch>
      </setting-item>
      <setting-item
        :disabled="config.launch.fullscreen"
        :title="$t('settings.game.windowSize')"
        :description="$t('settings.game.windowSizeDesc')"
        icon="window">
        <TextInputBox
          width="100px"
          style="display: inline-block; margin-right: 16px"
          :placeholder="$t('settings.game.windowSizeWidth')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.width">
        </TextInputBox>
        <TextInputBox
          width="100px"
          style="display: inline-block"
          :placeholder="$t('settings.game.windowSizeHeight')"
          :number-only="true"
          :disabled="config.launch.fullscreen"
          v-model.number="config.launch.height">
        </TextInputBox>
      </setting-item>
      <setting-item :title="$t('settings.game.hideLauncherAfterLaunch')" icon="eye-slash">
        <toggle-switch></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.autoRefreshAccount')"
        :description="$t('settings.game.autoRefreshAccountDesc')"
        icon="user-check">
        <toggle-switch v-model="config.launch.skip_refresh_account"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.autoCompleteGameFiles')"
        :description="$t('settings.game.autoCompleteGameFilesDesc')"
        icon="file-check">
        <toggle-switch v-model="config.launch.skip_check_files"></toggle-switch>
      </setting-item>
      <setting-item
        :title="$t('settings.game.demo')"
        :description="$t('settings.game.demoDesc')"
        icon="lock">
        <toggle-switch v-model="config.launch.is_demo"></toggle-switch>
      </setting-item>
    </setting-group>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import { Instance } from "@/types/instance";
import { computed } from "vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";

const props = defineProps<{
  instance: Instance;
}>();

const instanceName = computed(() => {
  return props.instance.config.name;
});

const config = useConfigStore();
</script>

<style lang="less" scoped>
.settings {
  display: flex;
  flex-direction: column;
}
</style>
