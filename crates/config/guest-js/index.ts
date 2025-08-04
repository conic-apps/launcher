import { invoke } from "@tauri-apps/api/core"

export async function loadConfigFile(value: string): Promise<string | null> {
    return await invoke<{ value?: string }>("plugin:example|ping", {
        payload: {
            value,
        },
    }).then((r) => (r.value ? r.value : null))
}
