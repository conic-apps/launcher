export type Instance = {
    config: {
        name: string
        runtime: {
            minecraft: string
            mod_loader_type: "Fabric" | "Quilt" | "Forge" | "Neoforge" | undefined
            mod_loader_version: string | undefined
        }
    }
    installed: boolean
}
