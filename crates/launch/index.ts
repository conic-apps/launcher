// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

import { Config } from "@conic/config"
import { DownloadState } from "@conic/download"
import { Instance } from "@conic/instance"
import { Channel, invoke } from "@tauri-apps/api/core"

type LaunchProgress =
    | {
          job: "Prepare"
      }
    | {
          job: "CompleteFiles"
          downloadState?: DownloadState
      }
    | {
          job: "GenerateScriptlet"
      }
    | {
          job: "WaitForLaunch"
      }
    | {
          job: "LogSettingUser"
      }
    | {
          job: "LogLwjglVersion"
      }
    | {
          job: "LogOpenALLoaded"
      }
    | {
          job: "LogTextureLoaded"
      }

/**
 * Usage:
 * ```ts
 * const task = new LaunchTask;
 * task.callbacks = {
 *   onProgress: ...
 * }
 * await task.start()
 * ```
 */
export class LaunchTask {
    private _config: Config
    private _instance: Instance
    private _callbacks?: {
        onProgress?: (task: LaunchProgress) => void
    }
    constructor(config: Config, instance: Instance, callbacks?: typeof this._callbacks) {
        this._config = config
        this._instance = instance
        this._callbacks = callbacks
    }
    async start() {
        const channel = new Channel<LaunchProgress>()
        channel.onmessage = (message) => {
            this._callbacks?.onProgress?.(message)
        }
        await invoke("plugin:launch|cmd_spawn_launch_task", {
            config: this._config,
            instance: this._instance,
            channel,
        })
    }
    async cancel() {
        await invoke("plugin:launch|cmd_cancel_launch_task")
    }
}
