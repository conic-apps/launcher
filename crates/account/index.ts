import { invoke } from "@tauri-apps/api/core"

export type Account = {
    refresh_token?: string
    access_token?: string
    token_deadline?: number
    profile: {
        avatar: string
        profile_name: string
        uuid: string
        skins: {
            id: string
            state: string
            textureKey: string
            url: string
            variant: string
        }[]
        capes: {
            alias: string
            id: string
            state: string
            url: string
        }[]
    }
    account_type: "Microsoft" | "Offline"
}

export async function listAccounts() {
    return await invoke("plugin:account|cmd_list_accounts", {}).then((r) => r as Account[])
}

export async function getAccountByUuid(uuid: string) {
    return await invoke("plugin:account|cmd_get_account_by_uuid", { payload: { uuid } }).then(
        (r) => r as Account[],
    )
}

export async function deleteAccount(uuid: string) {
    return await invoke("plugin:account|cmd_delete_account", { payload: { uuid } })
}

export async function addMicrosoftAccount(code: string) {
    return await invoke("plugin:account|cmd_add_microsoft_account", { payload: { code } })
}
