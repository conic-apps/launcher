import { invoke } from "@tauri-apps/api/core"

export type PlatformInfo = {
    arch: string
    arch_from_uname?: string
    os_type: string
    os_family: "Windows" | "Linux" | "Macos"
    os_version: NonNullable<any>
    edition?: string
}

export async function getPlatformInfo(): Promise<PlatformInfo> {
    return await invoke("plugin:platform|cmd_get_platform_info")
}
