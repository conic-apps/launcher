<template>
  <dialog-vue :visible="props.show" width="860" height="520">
    <div class="worlds">
      <div style="width: 100%; height: 100%">
        <div class="title">
          <div style="display: flex; align-items: center">
            <div class="icon">
              <i class="map"></i>
            </div>
            <div>
              <h4>{{ $t("game.gameData.saves") }}</h4>
              <p>{{ $tc("game.saves.description", { counts: props.datas.length }) }}</p>
            </div>
          </div>
          <div class="button" style="position: absolute; right: 0" @click="$emit('close')">
            <i></i>
          </div>
        </div>

        <div class="content">
          <div class="row1">
            <div>
              <list-item v-for="(world, index) in datas" :key="index" :title="world.levelData.LevelName"
                :logo="world.icon" :click-able="false"
                :buttons="['circle-info', 'folders', 'arrow-up-right-from-square']">
                <template #subtitle>
                  <tag :text="world.levelData.Version.Name" :color="['180', '180', '180']" text-color="#fffffff0"
                    :border="true" :round="true"></tag>
                  <tag v-if="world.levelData.allowCommands" :text="$t('game.saves.allowCheat')"
                    :color="['180', '180', '180']" text-color="#fffffff0" :border="true" :round="true"></tag>
                  <tag v-if="world.levelData.hardcore" :text="$t('game.saves.hardcore')" :color="['180', '180', '180']"
                    text-color="#fffffff0" :border="true" :round="true"></tag>
                </template>
                {{ world.folderName }}
              </list-item>
            </div>
          </div>
          <div class="row2">
            <p>{{ $t("game.saves.showMap") }}</p>
          </div>
        </div>
      </div>
    </div>
  </dialog-vue>
</template>

<script setup lang="ts">
import DialogVue from "@/components/Dialog.vue";
import ListItem from "@/components/ListItem.vue";
import Tag from "@/components/Tag.vue";

const props = defineProps<{
  show: boolean;
  instanceName: string;
  datas: Save[];
}>();

export type Save = {
  icon: string;
  levelData: any;
  folderName: string;
};
</script>

<style lang="less" scoped>
.worlds {
  width: 100%;
  height: 100%;
  overflow: hidden;

  .title {
    border-bottom: 1px solid #ffffff18;
    width: 100%;
    height: 80px;
    display: flex;
    justify-content: space-between;
    position: relative;

    .icon {
      width: 80px;
      height: 80px;
      display: flex;
      align-items: center;
      justify-content: center;

      .back {
        border-radius: 1000px;
        width: 40px;
        height: 40px;
        border: 1px solid rgba(255, 255, 255, 0.38);
      }

      .back:active {
        opacity: 0.7;
      }

      .back::before {
        font-size: 20px;
      }
    }

    i {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    i::before {
      font-size: 36px;
      font-style: normal;
      font-family: "fa-pro";
    }

    h4 {
      font-size: 22px;
      font-weight: normal;
    }

    p {
      font-size: 14px;
      margin-top: 4px;
      opacity: 0.7;
      font-weight: normal;
    }

    .button {
      width: 20px;
      height: 20px;
      border-radius: 50%;
      margin-left: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: transform 100ms;
      background: #ffffff40;

      i {
        transition: all 100ms ease;
      }

      i::before {
        content: "\f00d";
        font-size: 12px;
        margin-top: 1px;
        margin-left: 0.6px;
        font-style: normal;
        font-family: "fa-pro";
        opacity: 0;
        transition: all 70ms ease;
      }
    }

    .button:hover {
      i::before {
        opacity: 1;
      }
    }

    .button:active {
      i {
        opacity: 0.7;
      }
    }
  }

  .content {
    display: flex;
    height: calc(100% - 80px);
    padding-top: 16px;

    .row1 {
      width: 50%;
      height: 100%;
      padding: 0 12px;
      overflow: auto;

      >div {
        border-radius: 8px;
        overflow: hidden;
      }

      .list-item {
        width: 100%;
      }
    }

    .row2 {
      width: 50%;
      height: 100%;
      border: 1px solid rgba(255, 255, 255, 0.08);
      border-radius: 10px;
      display: flex;
      align-items: center;
      justify-content: center;

      p {
        font-style: italic;
        opacity: 0.6;
      }
    }
  }
}
</style>
