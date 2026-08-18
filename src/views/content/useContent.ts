import { ref } from "vue"

const showContent = ref({
    saves: false,
    mods: false,
    resourcepacks: false,
    screenshots: false,
})

const showContentDetails = ref({
    modrinth: {
        mod: null as string | null,
        resourcepack: null as string | null,
    },
    curseforge: {
        mod: null as string | null,
        resourcepack: null as string | null,
    },
})

export function useShowContent() {
    return showContent
}

export function useShowContentDetails() {
    return showContentDetails
}
