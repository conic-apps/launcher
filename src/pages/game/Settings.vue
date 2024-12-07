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
      <setting-item
        v-if="instanceName === 'Latest Release'"
        :title="$t('settings.accessibility.hideLatestRelease')"
        :description="$t('settings.accessibility.hideLatestReleaseDesc')"
        icon="eye-slash">
        <toggle-switch v-model="config.accessibility.hide_latest_release"></toggle-switch>
      </setting-item>
      <setting-item
        v-if="instanceName === 'Latest Snapshot'"
        :title="$t('settings.accessibility.hideLatestSnapshot')"
        :description="$t('settings.accessibility.hideLatestSnapshotDesc')"
        icon="eye-slash">
        <toggle-switch v-model="config.accessibility.hide_latest_snapshot"></toggle-switch>
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
    <setting-group :title="$t('settings.advance.launchArgs')">
      <setting-item :title="$t('settings.advance.gc')">
        <select-vue
          :display-name="['G1GC', 'ZGC', 'ParallelGC', 'ParallelOldGC', 'SerialGC']"
          :options="['G1', 'Z', 'Parallel', 'ParallelOld', 'Serial']"
          v-model="config.launch.gc"
          :default="0"></select-vue>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraJVMArgs')"
        :description="$t('settings.advance.extraJVMArgsDesc')">
        <TextInputBox width="300px" v-model="config.launch.extra_jvm_args"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraMinecraftArgs')"
        :description="$t('settings.advance.extraMinecraftArgsDesc')">
        <TextInputBox width="300px" v-model="config.launch.extra_mc_args"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.extraClassPaths')"
        :description="$t('settings.advance.extraClassPathsDesc')">
        <TextInputBox width="300px" v-model="config.launch.extra_class_paths"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.executeBeforeLaunch')"
        :description="$t('settings.advance.executeBeforeLaunchDesc')">
        <TextInputBox width="300px" v-model="config.launch.execute_before_launch"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.wrapCommand')"
        :description="$t('settings.advance.wrapCommandDesc')">
        <TextInputBox width="300px" v-model="config.launch.wrap_command"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.executeAfterLaunch')"
        :description="$t('settings.advance.executeAfterLaunchDesc')">
        <TextInputBox width="300px" v-model="config.launch.execute_after_launch"></TextInputBox>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.ignoreInvalidMinecraftCertificates')"
        :description="$t('settings.advance.ignoreInvalidMinecraftCertificatesDesc')">
        <ToggleSwitch v-model="config.launch.ignore_invalid_minecraft_certificates"></ToggleSwitch>
      </setting-item>
      <setting-item
        :title="$t('settings.advance.ignorePatchDiscrepancies')"
        :description="$t('settings.advance.ignorePatchDiscrepanciesDesc')">
        <ToggleSwitch v-model="config.launch.ignore_patch_discrepancies"></ToggleSwitch>
      </setting-item>
      <setting-item :title="$t('settings.advance.lwjglSettings')" description="" :clickAble="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
      <setting-item
        title="Open Log Viewer"
        description="The Description of Open Log Viewer "
        icon="scroll"
        :clickAble="true"
        @click="logViewerOpen = true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
    <setting-group title="Danger Zone" :danger="true">
      <setting-item
        title="Delete This Instance"
        description="Once you delete a instance, there is no going back. Please be certain."
        icon="trash-can"
        :clickAble="true"
        @click="confirmDeleteInstanceVisible = true"
        :disabled="instanceName === 'Latest Release' || instanceName === 'Latest Snapshot'">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
      <setting-item
        title="Reset This Instance"
        description="Clear all data in this instance, including worlds, packages, and modules"
        icon="arrow-rotate-right"
        :clickAble="true">
        <i class="chevron-right" style="margin-right: 10px"></i>
      </setting-item>
    </setting-group>
    <confirm-delete-instance
      :visible="confirmDeleteInstanceVisible"
      @close="confirmDeleteInstanceVisible = false"
      @deleted="
        confirmDeleteInstanceVisible = false;
        $emit('update-instance-list');
      "></confirm-delete-instance>
    <LogViewer :visible="logViewerOpen" @close="logViewerOpen = false"> </LogViewer>
  </div>
</template>

<script setup lang="ts">
import SettingItem from "@/components/SettingItem.vue";
import SettingGroup from "@/components/SettingGroup.vue";
import { useConfigStore } from "@/store/config";
import TextInputBox from "@/components/controllers/TextInputBox.vue";
import { computed, ref } from "vue";
import ToggleSwitch from "@/components/controllers/ToggleSwitch.vue";
import ConfirmDeleteInstance from "../dialogs/ConfirmDeleteInstance.vue";
import SelectVue from "@/components/controllers/Select.vue";
import LogViewer from "../dialogs/LogViewer.vue";
import { useInstanceStore } from "@/store/instance";

defineEmits(["update-instance-list"]);

const instanceStore = useInstanceStore();

const instanceName = computed(() => {
  return instanceStore.currentInstance.config.name;
});

const config = useConfigStore();

const confirmDeleteInstanceVisible = ref(false);

const logViewerOpen = ref(false);
</script>

<style lang="less" scoped>
.settings {
  display: flex;
  flex-direction: column;
}
</style>
