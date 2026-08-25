import { ref } from "vue"

const showContent = ref({
    saves: false,
    mods: false,
    resourcepacks: false,
    screenshots: false,
    packs: false,
})

const showContentDetails = ref({
    modrinth: {
        mod: null as string | null,
        resourcepack: null as string | null,
        pack: null as string | null,
    },
    curseforge: {
        mod: null as number | null,
        resourcepack: null as number | null,
        pack: null as number | null,
    },
})

export function useShowContent() {
    return showContent
}

export function useShowContentDetails() {
    return showContentDetails
}
