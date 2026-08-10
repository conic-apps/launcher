<!-- Conic Launcher -->
<!-- Copyright 2022-2026 ConicMC developers. All rights reserved. -->
<!-- SPDX-License-Identifier: GPL-3.0-only -->

<template>
  <div class="content-saves">
    <div class="title">
      <AppIcon name="extension-puzzle"></AppIcon>
      <p>模组列表</p>
      <div class="select-source">
        <button><AppIcon name="file-tray-full-outline"></AppIcon></button>
        <button>
          <Modrinth fill="var(--ctp-green)" style="width: 24px; padding: 3px"></Modrinth>
        </button>
        <button>
          <CurseForge fill="var(--ctp-peach)" style="width: 24px"></CurseForge>
        </button>
      </div>
    </div>
    <div class="save-list-wrapper">
      <div class="saves-list"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import Modrinth from "@/assets/images/modrinth.svg";
import CurseForge from "@/assets/images/curseforge.svg";
import AppIcon from "@/components/AppIcon.vue";
</script>

<style lang="less" scoped>
.content-saves {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  flex: 1;
  .save-list-wrapper {
    padding: 16px 32px 32px 32px;
  }
  .title {
    width: 100%;
    background: var(--ctp-mantle);
    height: 52px;
    padding: 0 32px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    gap: 8px;
    .select-source {
      display: flex;
      align-items: center;
      border-radius: 4px;
      overflow: hidden;
      margin-left: auto;
      border: 1px solid var(--ctp-surface1);
      button {
        appearance: none;
        border: none;
        background: none;
        background: var(--ctp-surface0);
        width: 40px;
        height: 32px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-right: 1px solid var(--ctp-surface1);
      }
      button:last-child {
        border-right: none;
      }
    }
  }
}
.saves-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  justify-content: center;
  width: 100%;
  row-gap: 12px;
  column-gap: 12px;
  .content {
    display: flex;
    border-radius: 8px;
    image-rendering: pixelated;
    transform: translateX(4px);
    background: rgba(var(--ctp-surface0-rgb), 0.4);
    img {
      border: 2px solid var(--ctp-surface0);
      border-radius: 8px 0 0 8px;
      transition: opacity 200ms ease;
    }
    .content-info {
      background: var(--ctp-surface0);
      padding: 8px 12px;
      transform: translateX(-8px);
      width: calc(100% - 64px);
      border-radius: 8px;
      transition: all 200ms ease;
      p.name {
        font-size: 14px;
      }
      p.folder-name {
        font-size: 10px;
        margin: 4px 0;
        opacity: 0.6;
      }
      span.game-mode,
      span.command-enabled {
        font-size: 9px;
        padding: 2px 6px;
        border-radius: 100px;
        font-weight: 500;
        color: var(--ctp-text-inverse);
      }
      span.game-mode.survival {
        background: var(--ctp-green);
      }
      span.game-mode.creative {
        background: var(--ctp-mauve);
      }
      span.game-mode.adventure {
        background: var(--ctp-peach);
      }
      span.game-mode.spectator {
        background: var(--ctp-blue);
      }
      span.command-enabled {
        background: var(--ctp-yellow);
        margin-left: 4px;
      }
      span.last-played {
        font-size: 10px;
        margin-left: 4px;
        span.label {
          opacity: 0.8;
        }
      }
    }
    .actions {
      position: absolute;
      right: 4px;
      top: 0;
      height: 100%;
      display: flex;
      flex-direction: column;
      padding: 12px 0;
      align-items: center;
      justify-content: space-between;
      z-index: -1;
      button {
        appearance: none;
        border: none;
        background: none;
        opacity: 0;
        transform: scale(0.5);
        transition:
          opacity 200ms ease,
          transform 200ms ease;
      }
    }
    .download-button {
      position: absolute;
      left: 20px;
      top: 50%;
      transform: translateY(-50%);
      button {
        appearance: none;
        background: none;
        border: none;
        opacity: 0;
        transform: scale(0.5);
        transition:
          opacity 200ms ease,
          transform 200ms ease;
      }
    }
  }
  .content:hover {
    .content-info {
      width: calc(100% - 80px);
    }
    .actions button {
      opacity: 0.8;
      transform: scale(1);
    }
    .actions button:hover {
      opacity: 1;
    }
    .actions button:active {
      opacity: 0.9;
    }
    img {
      opacity: 0.4;
    }
    .download-button button {
      opacity: 1;
      transform: scale(1);
    }
    img:active ~ .download-button button {
      opacity: 0.7;
      transition: opacity 55ms ease;
    }
  }
}
</style>
