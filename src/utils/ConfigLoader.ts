import { invoke } from "@tauri-apps/api"

export async function load(key: string): Promise<any> {
    // return await invoke('getConfig', { key: key })
    return false
}

export async function update(key: string, value: any): Promise<any> {
    // invoke('updateConfig', { key: key, value: value })
}

export function reset() { }