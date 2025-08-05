import { Config } from "@conic/config"
import { Instance } from "@conic/instance"
import { invoke } from "@tauri-apps/api/core"

export async function launch(config: Config, instance: Instance) {
    return await invoke("plugin:launch|cmd_launch", { payload: { config, instance } })
}
